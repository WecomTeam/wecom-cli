use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::skip_serializing_none;

use crate::common::constants;
use crate::{Error, Result};

/// Long-task polling status: completion flag and polling metadata.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LongTaskPollInfo {
    pub done: Option<bool>,
    pub task_timeout: Option<u64>,
    pub polling_interval_ms: Option<u64>,
    #[serde(flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}

/// Trait for polling data sources: provides long-task polling status.
pub trait LongTaskPollData {
    fn poll_info(&self) -> Option<LongTaskPollInfo>;
}

/// Long-task polling wire mode, decided by the first response that carries `taskid`.
///
/// Serialized as `0` (TaskQuery) / `1` (ReuseEndpoint) to match the backend wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PollMode {
    /// TaskQuery: POST /task/query with {method, payload}.
    #[default]
    TaskQuery = 0,
    /// ReuseEndpoint: reuse the original request endpoint, empty JSON body,
    /// carry taskid via the `X-Long-Poll-TaskId` header.
    ReuseEndpoint = 1,
}

/// Generic long-task polling framework.
///
/// Abstracts the common logic (retry, backoff, timeout, interval) shared by
/// transports. The caller injects "send request" and "parse
/// response" via a closure.
///
/// ## Tracing
/// Opens a `long_task.poll` span wrapping the entire polling loop with
/// `taskid` as a field. Errors are recorded at WARN level.
#[tracing::instrument(level = "info", name = "long_task.poll", skip_all)]
pub async fn poll_long_task<T, F, Fut>(mut fetch: F) -> Result<T>
where
    T: LongTaskPollData + serde::Serialize,
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let start = std::time::Instant::now();
    let mut timeout: Option<std::time::Duration> = None;
    let mut consecutive_network_err_count: u32 = 0;

    loop {
        let data = match fetch().await {
            Ok(data) => data,
            Err(e) => {
                if !matches!(&e, Error::Network { .. }) {
                    tracing::error!(
                        error = %e,
                        "long task poll non-network error, propagating immediately",
                    );
                    return Err(e);
                }

                consecutive_network_err_count += 1;
                if consecutive_network_err_count > constants::MAX_CONSECUTIVE_NETWORK_ERRORS {
                    tracing::error!(
                        count = %consecutive_network_err_count,
                        max = constants::MAX_CONSECUTIVE_NETWORK_ERRORS,
                        "long task poll exceeded max consecutive network errors, giving up",
                    );
                    return Err(e);
                }
                tracing::info!(
                    count = %consecutive_network_err_count,
                    error = %e,
                    "long task poll network error, retrying",
                );
                tokio::time::sleep(std::time::Duration::from_millis(
                    constants::DEFAULT_MIN_POLL_INTERVAL_MS
                        * 2u64.pow(consecutive_network_err_count),
                ))
                .await;
                continue;
            }
        };

        consecutive_network_err_count = 0;

        let task_info = data
            .poll_info()
            .ok_or_else(|| Error::Parse {
                message: "Missing 'long_task_poll' field".into(),
                endpoint: "PollClawLongTask".into(),
                body: Box::new(serde_json::to_value(&data).unwrap_or_default()),
                source: None,
            })
            .inspect_err(|_| {
                tracing::error!(
                    error = "Missing 'long_task_poll' field in poll response",
                    "poll response missing long_task_poll field"
                );
            })?;

        if task_info.done == Some(true) {
            tracing::info!("long task poll completed");
            return Ok(data);
        }

        if timeout.is_none() {
            timeout = Some(std::time::Duration::from_secs(
                task_info
                    .task_timeout
                    .unwrap_or(constants::DEFAULT_POLL_TIMEOUT_SECS),
            ));
        }

        if let Some(t) = timeout
            && start.elapsed() >= t
        {
            tracing::error!(
                timeout_secs = t.as_secs(),
                elapsed_secs = start.elapsed().as_secs(),
                "long task poll timed out",
            );
            return Err(Error::Other(
                format!("轮询任务超时: {}s", t.as_secs()).into(),
            ));
        }

        let interval = task_info
            .polling_interval_ms
            .unwrap_or(constants::DEFAULT_MIN_POLL_INTERVAL_MS)
            .max(constants::DEFAULT_MIN_POLL_INTERVAL_MS);
        tracing::debug!(
            interval_ms = interval,
            "long task poll iteration, waiting for next round",
        );
        tokio::time::sleep(std::time::Duration::from_millis(interval)).await;
    }
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：polling（通用长任务轮询框架）
    //!
    //! ### 关键接口
    //! - [poll_long_task] — 通用轮询循环：重试、退避、超时、间隔，直到 done=true
    //! - [LongTaskPollData::poll_info] — 从业务数据中提取轮询元信息（done/timeout/interval）
    //! - [LongTaskPollInfo] — 轮询状态结构体：done、task_timeout、polling_interval_ms
    //!
    //! ### 关键分支与异常路径
    //! - 首次即返回 done=true → 立即返回 Ok
    //! - 循环多次直到 done=true → 按 polling_interval_ms 休眠后继续
    //! - poll_info() 返回 None → 返回 Error::Parse("Missing 'long_task_poll'")
    //! - 非 Network 错误（Api/Http）→ 立即传播不重试
    //! - Network 错误（任意类型）→ 指数退避重试，超过上限则返回 Err
    //!   （轮询为只读 task/query，无副作用，因此不做错误类型细分）
    //! - task_timeout 超时 → 返回 Error::Other(timeout message)
    //!
    //! ### 上下游交互
    //! - 上游：[http::long_task] 调用 [poll_long_task] 并注入 fetch 闭包
    //! - 下游：依赖 [fetch] 闭包执行实际 HTTP 请求；错误类型使用 crate::Error
    //!
    //! ### 测试范围分层（与 http/long_task 的分工）
    //! 本模块仅测试**框架层纯逻辑**（通过注入 MockPollData 闭包模拟）：
    //! - 轮询循环、done 判定、超时与间隔计算
    //! - 错误分类（任意 Network 重试 vs 非 Network 立即传播）
    //! - 指数退避 / 最大重试次数 / 网络恢复
    //! - LongTaskPollInfo 序列化
    //!
    //! 协议适配层（http/long_task）负责通过 wiremock 覆盖：
    //! - 真实 HTTP 请求构造与 URL 路径拼接
    //! - 响应解析（成功 / 业务错误 / HTTP 错误 / 非法 JSON）
    //! - Headers / payload 结构
    //!
    //! 两层测试**不可互相替代**：框架层用例虽然覆盖了轮询循环逻辑，但无法验证真实 I/O 链路；
    //! 反之，协议层用例聚焦于 wire 上的格式，不重复测框架层的重试/超时等纯逻辑。

    use assert_json_diff::assert_json_eq;
    use serde_json::json;

    use super::*;
    use crate::Error;

    // ── LongTaskPollInfo 序列化/反序列化 ──

    /// P0：[LongTaskPollInfo] LongTaskPollInfo 完整字段序列化/反序列化往返一致
    /// 条件：构造 done/timeout/interval 均有值的实例
    /// 断言：序列化 JSON 包含所有字段且反序列化后等价
    #[test]
    fn poll_info_full_serialization_roundtrip() {
        let info = LongTaskPollInfo {
            done: Some(true),
            task_timeout: Some(60),
            polling_interval_ms: Some(1000),
            ..Default::default()
        };
        let json = serde_json::to_value(&info).unwrap();

        assert_json_eq!(
            json,
            json!({
                "done": true,
                "task_timeout": 60,
                "polling_interval_ms": 1000,
            })
        );

        let deserialized: LongTaskPollInfo = serde_json::from_value(json.clone()).unwrap();
        assert_json_eq!(serde_json::to_value(&deserialized).unwrap(), json);
    }

    /// P1：[LongTaskPollInfo] LongTaskPollInfo 全字段为 None 时序列化为空对象
    /// 条件：done/timeout/interval 全部为 None
    /// 断言：JSON 为 {}，反序列化后各字段仍为 None
    #[test]
    fn poll_info_all_none_serializes_to_empty_object() {
        let info = LongTaskPollInfo {
            done: None,
            task_timeout: None,
            polling_interval_ms: None,
            ..Default::default()
        };
        let json = serde_json::to_value(&info).unwrap();

        // match 语义：全 None → 空对象
        assert_json_eq!(json, json!({}));

        let deserialized: LongTaskPollInfo = serde_json::from_value(json).unwrap();
        assert!(deserialized.done.is_none());
        assert!(deserialized.task_timeout.is_none());
        assert!(deserialized.polling_interval_ms.is_none());
    }

    /// P1：LongTaskPollInfo 部分字段时只输出有值字段
    /// 条件：仅 done=false，其余为 None
    /// 断言：JSON 仅包含 "done": false
    #[test]
    fn poll_info_partial_fields_serialize_correctly() {
        let info = LongTaskPollInfo {
            done: Some(false),
            task_timeout: None,
            polling_interval_ms: None,
            ..Default::default()
        };
        let json = serde_json::to_value(&info).unwrap();

        // match 语义：只包含 done 字段
        assert_json_eq!(json, json!({ "done": false }));
    }

    // ── LongTaskPollData trait ──

    #[derive(Debug, Clone)]
    struct MockPollData {
        info: Option<LongTaskPollInfo>,
    }

    impl LongTaskPollData for MockPollData {
        fn poll_info(&self) -> Option<LongTaskPollInfo> {
            self.info.clone()
        }
    }

    impl serde::Serialize for MockPollData {
        fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            use serde::ser::SerializeStruct;
            let s = serializer.serialize_struct("MockPollData", 0)?;
            s.end()
        }
    }

    // ── poll_long_task 集成测试 ──

    /// P0：[poll_long_task] poll_long_task 首次即返回完成数据
    /// 条件：fetch 闭包首次返回 done=true 的 MockPollData
    /// 断言：结果 poll_info().done 为 true
    #[tokio::test]
    async fn poll_immediately_done_returns_data() {
        let data = MockPollData {
            info: Some(LongTaskPollInfo {
                done: Some(true),
                task_timeout: None,
                polling_interval_ms: None,
                ..Default::default()
            }),
        };

        let result: MockPollData = poll_long_task(|| async { Ok(data.clone()) }).await.unwrap();

        assert_eq!(result.poll_info().unwrap().done, Some(true));
    }

    /// P0：[poll_long_task] poll_long_task 循环轮询直到完成
    /// 条件：前两次 fetch 返回 done=false，第三次 done=true
    /// 断言：最终 done=true 且调用次数 >= 3
    #[tokio::test(start_paused = true)]
    async fn poll_polls_until_done() {
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

        let result: MockPollData = poll_long_task(|| {
            let count = call_count.clone();
            async move {
                let n = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n < 2 {
                    // 前两次返回未完成（使用极短间隔加速测试）
                    Ok(MockPollData {
                        info: Some(LongTaskPollInfo {
                            done: Some(false),
                            task_timeout: None,
                            polling_interval_ms: Some(1), // 1ms 加速
                            ..Default::default()
                        }),
                    })
                } else {
                    // 第三次完成
                    Ok(MockPollData {
                        info: Some(LongTaskPollInfo {
                            done: Some(true),
                            task_timeout: None,
                            polling_interval_ms: None,
                            ..Default::default()
                        }),
                    })
                }
            }
        })
        .await
        .unwrap();

        assert_eq!(result.poll_info().unwrap().done, Some(true));
        assert!(call_count.load(std::sync::atomic::Ordering::SeqCst) >= 3);
    }

    /// P1：[poll_long_task] poll_long_task 对非 Network 的 Parse 错误立即传播
    /// 条件：fetch 闭包直接返回 Error::Parse（模拟外部解析失败）
    /// 断言：错误原样传播，类型为 Error::Parse 且消息包含 "long_task_poll"
    #[tokio::test]
    async fn poll_missing_long_task_poll_field_returns_error() {
        // 返回没有 long_task_poll 信息的数据
        let err: Error = poll_long_task::<MockPollData, _, _>(|| async {
            Err(Error::Parse {
                message: "Missing 'long_task_poll' field".into(),
                endpoint: "test".into(),
                body: Box::new(serde_json::Value::Null),
                source: None,
            })
        })
        .await
        .unwrap_err();

        match &err {
            Error::Parse { message, .. } => assert!(message.contains("long_task_poll")),
            other => panic!("Expected Parse error, got: {other:?}"),
        }
    }

    /// P1：[poll_long_task] poll_long_task 对非 Network 错误立即传播不重试
    /// 条件：fetch 返回 Error::Api (code=403)
    /// 断言：错误直接传播，匹配 Error::Api { code: Some(403) }
    #[tokio::test]
    async fn poll_non_network_error_propagates_immediately() {
        // 非 Network 错误应立即返回，不重试
        let err: Error = poll_long_task::<MockPollData, _, _>(|| async {
            Err(Error::Api {
                message: "forbidden".into(),
                action: "test".into(),
                code: Some(403),
                body: Box::new(serde_json::Value::Null),
            })
        })
        .await
        .unwrap_err();

        assert!(matches!(
            err,
            Error::Api {
                code: Some(403),
                ..
            }
        ));
    }

    /// P1：[poll_long_task] poll_long_task 对 HTTP 错误立即传播不重试
    /// 条件：fetch 返回 Error::Http (status=502)
    /// 断言：错误直接传播，匹配 Error::Http { status: 502 }
    #[tokio::test]
    async fn poll_http_error_propagates_immediately() {
        let err: Error = poll_long_task::<MockPollData, _, _>(|| async {
            Err(Error::Http {
                message: "502".into(),
                endpoint: "test".into(),
                status: 502,
            })
        })
        .await
        .unwrap_err();

        assert!(matches!(err, Error::Http { status: 502, .. }));
    }

    /// P1：[poll_long_task] poll_long_task 当 poll_info() 返回 None 时返回 Parse 错误
    /// 条件：fetch 返回 info 为 None 的 MockPollData
    /// 断言：错误类型为 Error::Parse 且消息包含 "long_task_poll"
    #[tokio::test]
    async fn poll_missing_poll_info_returns_parse_error() {
        // poll_info() 返回 None 时，应返回 Parse 错误
        let err: Error =
            poll_long_task::<MockPollData, _, _>(|| async { Ok(MockPollData { info: None }) })
                .await
                .unwrap_err();

        match &err {
            Error::Parse { message, .. } => assert!(message.contains("long_task_poll")),
            other => panic!("Expected Parse error, got: {other:?}"),
        }
    }

    // ── 超时与重试场景 ──

    /// P1：[poll_long_task] poll_long_task 超时返回 Error::Other
    /// 条件：首次轮询返回 done=false 且 task_timeout=0（立即超时）
    /// 断言：错误类型为 Error::Other，消息包含 "timeout"
    #[tokio::test]
    async fn poll_long_task_timeout() {
        let result: Result<MockPollData> = poll_long_task(|| async {
            // 第一次返回 done=false 且 timeout=0，触发立即超时
            Ok(MockPollData {
                info: Some(LongTaskPollInfo {
                    done: Some(false),
                    task_timeout: Some(0), // 0 秒超时，elapsed >= 0 立即成立
                    polling_interval_ms: Some(1),
                    ..Default::default()
                }),
            })
        })
        .await;

        let err = result.unwrap_err();
        match &err {
            Error::Other(msg) => {
                let msg_str = msg.to_string();
                assert!(
                    msg_str.contains("超时"),
                    "expected timeout message, got: {msg_str}"
                );
            }
            _ => panic!("Expected Error::Other(timeout), got: {err:?}"),
        }
    }

    /// 辅助函数：通过连接一个已关闭的端口产生真实的 reqwest::Error（is_connect() == true）
    async fn make_connect_error() -> reqwest::Error {
        // 绑定一个端口然后立刻 drop listener，确保该端口上无服务，产生 connection refused
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}"))
            .send()
            .await
            .unwrap_err()
    }

    /// P1：[poll_long_task] 连续网络错误超过 MAX_CONSECUTIVE_NETWORK_ERRORS (3) 时返回最后一个网络错误
    /// 条件：fetch 连续返回 4 次 Error::Network（超过 MAX=3）
    /// 断言：错误类型为 Error::Network
    #[tokio::test(start_paused = true)]
    async fn poll_long_task_network_retry_exceeds_max() {
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

        let result: Result<MockPollData> = poll_long_task(move || {
            let count = call_count.clone();
            async move {
                let n = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                // 前 4 次返回网络错误（MAX_CONSECUTIVE_NETWORK_ERRORS=3，第 4 次触发返回）
                if n < 4 {
                    Err(Error::Network {
                        message: "mock network failure".into(),
                        endpoint: "test".into(),
                        source: make_connect_error().await,
                    })
                } else {
                    Ok(MockPollData { info: None })
                }
            }
        })
        .await;

        let err = result.unwrap_err();
        match &err {
            Error::Network { message, .. } => {
                assert_eq!(message, "mock network failure");
            }
            _ => panic!("Expected Error::Network, got: {err:?}"),
        }
    }

    /// P1：[poll_long_task] 可重试网络错误后成功恢复
    /// 条件：前 2 次返回网络错误，第 3 次成功返回 done=true
    /// 断言：返回 Ok，done=true
    #[tokio::test(start_paused = true)]
    async fn poll_long_task_network_retry_recovery() {
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

        let result: MockPollData = poll_long_task(move || {
            let count = call_count.clone();
            async move {
                let n = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n < 2 {
                    Err(Error::Network {
                        message: "temp network error".into(),
                        endpoint: "test".into(),
                        source: make_connect_error().await,
                    })
                } else {
                    Ok(MockPollData {
                        info: Some(LongTaskPollInfo {
                            done: Some(true),
                            task_timeout: None,
                            polling_interval_ms: Some(1),
                            ..Default::default()
                        }),
                    })
                }
            }
        })
        .await
        .unwrap();

        assert_eq!(result.info.unwrap().done, Some(true));
    }
}
