use indexmap::IndexMap;
use tracing::field::Empty;
use wecom_transport::{HttpRequestPayload, TransportResponse};

use super::{MethodHandle, RunOptions, output, schema_util};
use crate::{CliRunOutput, Error, Result, directive, fs, schema};

// ─── Main execute function ──────────────────────────────────────

/// Execute a method call and handle its response.
///
/// Covers request preparation, directives, media uploads, pagination,
/// and output routing (file / stdout, binary vs JSON).
///
/// ## Tracing
/// Opens a `service.execute` span with `action` and `paged` fields.
/// Downstream directive, media-upload, and transport events are nested
/// under this span for per-call correlation. Errors are recorded at WARN level.
#[tracing::instrument(
    level = "info",
    name = "service.execute",
    skip_all,
    fields(action = %method.path().join("."), paged = Empty),
)]
pub(super) async fn execute_and_output(
    method: &MethodHandle<'_>,
    options: RunOptions<'_>,
) -> Result<()> {
    let client = method.client;

    let transport_backend = client.transport().name();
    tracing::info!(transport = transport_backend, "executing method");

    // 收集请求 directives
    let request_schema = method.request_schema();
    let (directives, multipart) = if let Some(schema) = &request_schema {
        let directives = directive::collect_directives(
            &method.service_schema.schemas,
            schema,
            &options.payload,
            options.run.get_cwd(),
        );
        let multipart = directive::check_has_octet_stream(&method.service_schema.schemas, schema);
        tracing::debug!(
            directives_count = directives.len(),
            multipart,
            "request directives collected",
        );
        (directives, multipart)
    } else {
        (vec![], false)
    };

    let fs = std::sync::Arc::clone(options.run.get_fs());

    // 预留 / 校验 output 路径（首页请求之前：提前暴露写权限问题，fail fast）
    let mut output_file = match options.output_path.clone() {
        Some(path) => {
            let corrected = fs.resolve(&path, fs::FsAccess::Write).await?;
            Some(output::create_output_file(fs.as_ref(), &corrected).await?)
        }
        None => None,
    };

    let result = run_request(method, options, &directives, multipart, &mut output_file).await;

    // 唯一回退点：核心失败且预留文件未被保留 → 经写端 discard 撤回（不留空壳 /
    // 半成品）。保留的（已 keep）文件随作用域正常关闭，不删。
    if result.is_err()
        && let Some(info) = output_file.take()
        && !info.is_kept()
    {
        info.rollback().await;
    }

    result
}

/// Execute the request and route its response to the output destination.
///
/// Receives the pre-reserved `output_file` and marks it kept at the point
/// where the written content counts as a valid artifact (first NDJSON page
/// written, JSON body synced, binary stream completed); the caller rolls back
/// any reservation left unkept after a failure.
async fn run_request(
    method: &MethodHandle<'_>,
    mut options: RunOptions<'_>,
    directives: &[directive::Directive<'_>],
    multipart: bool,
    output_file: &mut Option<output::OutputFileInfo>,
) -> Result<()> {
    let client = method.client;

    let output = options.run.get_output();
    let fs = options.run.get_fs();

    // Retain the authorized directory for every subsequent output consumer.
    if let Some(path) = options.output_dir.clone() {
        let resolved = fs
            .resolve(&path, fs::FsAccess::WriteDir)
            .await
            .inspect_err(|e| tracing::error!(error = %e, "sandbox check dir writable failed"))?;
        options.output_dir = Some(resolved);
    }

    // 处理 media upload
    if !directives.is_empty() {
        tracing::debug!(
            directives_count = directives.len(),
            "processing media upload directives",
        );
        directive::process_media_upload(
            client,
            fs,
            &mut options.payload,
            directives,
            options.run.get_options(),
        )
        .await?;
    }

    // 检查 form-data 上传文件总大小不超过 100 MB
    check_multipart_body_size(fs.as_ref(), directives, 100 * 1024 * 1024).await?;

    // ── 第一页请求 ──────────────────────────────────────────

    let on_extra_data = options.run.get_on_extra_data();

    let payload = if multipart {
        // multipart 经工厂包装（延迟物化）：每次发送/重放时重新打开文件构建独立表单。
        let fs = std::sync::Arc::clone(fs);
        let payload = options.payload.clone();
        let file_fields = directive::multipart_file_fields(directives);
        HttpRequestPayload::form(move || {
            let fs = std::sync::Arc::clone(&fs);
            let payload = payload.clone();
            let file_fields = file_fields.clone();
            async move {
                directive::build_multipart_form(fs.as_ref(), &payload, &file_fields)
                    .await
                    .map_err(crate::util::to_transport_error)
            }
        })
    } else {
        // JSON 直值：构造时一次 Arc，重放零拷贝。
        HttpRequestPayload::json(options.payload.clone())
    };

    let request = client
        .transport()
        .invoke(method.endpoint(), payload)
        .with_options(options.run.get_options().clone());

    let mut data = match request.execute().await? {
        TransportResponse::Json(output) => {
            tracing::debug!(extra_count = output.extra.len(), "JSON response received");
            if let Some(cb) = on_extra_data
                && !output.extra.is_empty()
            {
                cb(&output.extra);
            }
            output.result
        }
        TransportResponse::Binary(response) => {
            tracing::info!(
                content_length = response.content_length(),
                "binary response received, writing to output"
            );
            // 二进制响应：不分页，二进制流落盘
            let result = output::handle_binary_output(
                &options,
                response,
                output_file.as_mut(),
                &method.method_path_segments,
            )
            .await?;

            output.print(&serde_json::to_string_pretty(&result).unwrap_or_default());
            return Ok(());
        }
    };

    // ── JSON 响应后续处理 ───────────────────────────────────

    let response_schema =
        schema_util::resolve_schema_ref(&method.service_schema.schemas, &method.schema.response);

    process_response_directives(
        &options,
        &mut data,
        &method.service_schema.schemas,
        &response_schema,
    )
    .await?;

    // 单页模式：直接输出
    if multipart || options.page_count.is_none() {
        tracing::info!(
            reason = if multipart {
                "multipart"
            } else {
                "no page count"
            },
            "single page mode, outputting result",
        );
        let result = output::handle_json_output(data, output_file.as_mut()).await?;

        output.print(&serde_json::to_string_pretty(&result).unwrap_or_default());
        return Ok(());
    }

    // 分页模式：NDJSON 追加写入
    tracing::Span::current().record("paged", true);
    tracing::info!(max_pages = options.page_count, "entering paged mode",);

    // 确定写入目标：--output > stdout
    write_page(output, &data, output_file).await?;

    // 首页已写入即为有效部分结果：后续分页失败也保留（R2）。
    if let Some(file) = output_file.as_mut() {
        file.keep();
    }

    // 翻页循环（从第 2 页开始）
    let mut total_pages: u32 = 1;
    for (n, _) in (1..options.page_count.unwrap_or(1)).enumerate() {
        let Some(next_cursor) = extract_next_cursor(&data) else {
            tracing::info!(
                total_pages = total_pages,
                "no next cursor, pagination stopped early",
            );
            break;
        };

        tracing::info!(page = %n + 2, cursor = %next_cursor, "Fetching page");

        // 页间延迟
        tokio::time::sleep(std::time::Duration::from_millis(options.page_delay_ms)).await;

        if let Some(obj) = options.payload.as_object_mut() {
            obj.insert("cursor".to_string(), serde_json::Value::String(next_cursor));
        }

        let page_output = client
            .transport()
            .invoke(method.endpoint(), &options.payload)
            .with_options(options.run.get_options().clone())
            .execute()
            .await?
            .into_json()?;
        if let Some(cb) = on_extra_data
            && !page_output.extra.is_empty()
        {
            cb(&page_output.extra);
        }
        data = page_output.result;

        process_response_directives(
            &options,
            &mut data,
            &method.service_schema.schemas,
            &response_schema,
        )
        .await?;
        write_page(output, &data, output_file).await?;
        total_pages += 1;
    }

    tracing::info!(total_pages = total_pages, "pagination complete");

    // 分页完成，如果写入了文件则打印路径信息
    if let Some(output_file) = output_file.as_mut() {
        output.print(
            &serde_json::to_string_pretty(&output_file.result_ndjson().await).unwrap_or_default(),
        );
    }

    Ok(())
}

/// 校验 form-data（`UploadMultipart`）文件总大小不超过 `limit`。
///
/// 逐个获取文件元信息并累加大小，任一文件超过 `limit` 即返回错误。
async fn check_multipart_body_size(
    fs: &dyn fs::Fs,
    directives: &[directive::Directive<'_>],
    limit: u64,
) -> Result<()> {
    let file_paths: Vec<_> = directives
        .iter()
        .filter_map(|d| match d {
            directive::Directive::UploadMultipart { file_path, .. } => Some(file_path.clone()),
            _ => None,
        })
        .collect();

    if file_paths.is_empty() {
        return Ok(());
    }

    let mut total_size: u64 = 0;

    for file_path in &file_paths {
        let file_size = fs.metadata(file_path).await?.len;
        total_size = total_size.saturating_add(file_size);

        if total_size > limit {
            return Err(Error::validation(format!(
                "请求文件总大小超过限制（{:.1} MB）: {}",
                limit as f64 / 1_048_576.0,
                file_path.display()
            )));
        }
    }

    Ok(())
}

/// 输出一页 NDJSON 数据：有文件则追加写入，否则通过 output.print() 输出。
async fn write_page(
    output: &CliRunOutput,
    data: &serde_json::Value,
    output_file: &mut Option<output::OutputFileInfo>,
) -> Result<()> {
    if let Some(f) = output_file {
        f.write_line(&serde_json::to_string(data).unwrap_or_default())
            .await?;
    } else {
        output.print(&data.to_string());
    }
    Ok(())
}

/// 对 JSON 响应执行 file-save 指令。
async fn process_response_directives(
    options: &RunOptions<'_>,
    data: &mut serde_json::Value,
    schemas: &IndexMap<String, schema::JsonSchema>,
    response_schema: &Option<schema::JsonSchema>,
) -> Result<()> {
    let Some(schema) = response_schema else {
        return Ok(());
    };
    for directive in &directive::collect_directives(schemas, schema, data, options.run.get_cwd()) {
        directive::process_file_save(options, data, directive).await?;
    }
    Ok(())
}

/// Inspect a JSON response and extract the next-page cursor.
///
/// Returns `Some(cursor)` when the response carries `has_more: true` **and**
/// a non-empty `next_cursor` string; otherwise returns `None`.
fn extract_next_cursor(data: &serde_json::Value) -> Option<String> {
    let has_more = data
        .get("has_more")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    if !has_more {
        return None;
    }

    data.get("next_cursor")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：execute（请求执行与输出处理）
    //!
    //! ### 关键接口
    //! - [execute_and_output] — 执行请求并处理响应（JSON/二进制、分页），失败时回退未保留的 --output
    //! - [extract_next_cursor] — 从 JSON 响应中提取下一页游标
    //! - [write_page] — 输出一页 NDJSON 数据
    //!
    //! ### 关键分支与异常路径
    //! - execute_and_output：HTTP 传输分支；JSON/二进制响应分支；单页/分页分支
    //! - 首页请求 / media upload 失败 → 未保留的 --output 预留文件回退（R1）
    //! - 分页首页写入成功后失败 → 文件已 keep，保留部分 NDJSON（R2）
    //! - 二进制流失败 → 未 keep → 回退（R3）
    //! - extract_next_cursor：has_more 为 false、next_cursor 为空、缺少字段等分支
    //!
    //! ### 上下游交互
    //! - 上游：[handler::handle_service_cmd] 调用本模块执行请求
    //! - 下游：依赖 [wecom_transport::Transport]、[output] 模块、[directive] 模块

    use serde_json::json;
    use wecom_fs::Fs;

    use super::*;
    use crate::directive::Directive;
    use crate::json_path::PathSegment;

    // extract_next_cursor

    /// P0：[extract_next_cursor] 在 has_more=true 且有 next_cursor 时返回游标
    /// 条件：JSON 中 has_more 为 true，next_cursor 为 "abc123"
    /// 断言：返回 Some("abc123")
    #[test]
    fn extract_cursor_has_more_true_with_cursor() {
        let data = json!({"has_more": true, "next_cursor": "abc123"});
        assert_eq!(extract_next_cursor(&data), Some("abc123".into()));
    }

    /// P1：[extract_next_cursor] 在 has_more=false 时返回 None
    /// 条件：JSON 中 has_more 为 false（有 next_cursor）
    /// 断言：返回 None
    #[test]
    fn extract_cursor_has_more_false() {
        let data = json!({"has_more": false, "next_cursor": "abc123"});
        assert_eq!(extract_next_cursor(&data), None);
    }

    /// P1：[extract_next_cursor] 在 has_more=true 但 next_cursor 为空时返回 None
    /// 条件：JSON 中 has_more 为 true，next_cursor 为 ""
    /// 断言：返回 None（空游标视为无效）
    #[test]
    fn extract_cursor_has_more_true_empty_cursor() {
        let data = json!({"has_more": true, "next_cursor": ""});
        assert_eq!(extract_next_cursor(&data), None);
    }

    /// P1：[extract_next_cursor] 在缺少 has_more 字段时返回 None
    /// 条件：JSON 中只有 next_cursor，无 has_more
    /// 断言：返回 None（has_more 默认 false）
    #[test]
    fn extract_cursor_no_has_more_field() {
        let data = json!({"next_cursor": "abc"});
        assert_eq!(extract_next_cursor(&data), None);
    }

    /// P1：[extract_next_cursor] 在缺少 next_cursor 字段时返回 None
    /// 条件：JSON 中 has_more 为 true，无 next_cursor 字段
    /// 断言：返回 None
    #[test]
    fn extract_cursor_no_next_cursor_field() {
        let data = json!({"has_more": true});
        assert_eq!(extract_next_cursor(&data), None);
    }

    /// P1：[extract_next_cursor] 在空 JSON 对象时返回 None
    /// 条件：JSON 为 {}
    /// 断言：返回 None
    #[test]
    fn extract_cursor_empty_object() {
        assert_eq!(extract_next_cursor(&json!({})), None);
    }

    // ── check_multipart_body_size ──

    /// P0：[check_multipart_body_size] 无 UploadMultipart 指令时返回 Ok
    /// 条件：directives 为空或不含 UploadMultipart
    /// 断言：返回 Ok(())
    #[tokio::test]
    async fn check_multipart_body_size_empty_directives_ok() {
        let fs = wecom_fs::SandboxedFs::new();
        let result = check_multipart_body_size(&fs, &[], 1024).await;
        assert!(result.is_ok());
    }

    /// P0：[check_multipart_body_size] 文件总大小在限制内时返回 Ok
    /// 条件：两个文件共 20 字节，limit = 1024
    /// 断言：返回 Ok(())
    #[tokio::test]
    async fn check_multipart_body_size_under_limit_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = wecom_fs::SandboxedFs::new();
        let file_a = tmp.path().join("a.txt");
        let file_b = tmp.path().join("b.txt");
        fs.atomic_write(&file_a, b"1234567890", Some(0o644))
            .await
            .unwrap();
        fs.atomic_write(&file_b, b"abcdefghij", Some(0o644))
            .await
            .unwrap();

        let directives = vec![
            Directive::UploadMultipart {
                path: vec![PathSegment::Key("file_a".into())],
                file_path: file_a.clone(),
            },
            Directive::UploadMultipart {
                path: vec![PathSegment::Key("file_b".into())],
                file_path: file_b.clone(),
            },
        ];

        let result = check_multipart_body_size(&fs, &directives, 1024).await;
        assert!(result.is_ok());
    }

    /// P0：[check_multipart_body_size] 文件总大小超过限制时返回 Err
    /// 条件：两个文件共 20 字节，limit = 15
    /// 断言：返回 Err，错误信息包含大小和限制
    #[tokio::test]
    async fn check_multipart_body_size_exceeds_limit_err() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = wecom_fs::SandboxedFs::new();
        let file_a = tmp.path().join("a.txt");
        let file_b = tmp.path().join("b.txt");
        fs.atomic_write(&file_a, b"1234567890", Some(0o644))
            .await
            .unwrap();
        fs.atomic_write(&file_b, b"abcdefghij", Some(0o644))
            .await
            .unwrap();

        let directives = vec![
            Directive::UploadMultipart {
                path: vec![PathSegment::Key("file_a".into())],
                file_path: file_a.clone(),
            },
            Directive::UploadMultipart {
                path: vec![PathSegment::Key("file_b".into())],
                file_path: file_b.clone(),
            },
        ];

        let result = check_multipart_body_size(&fs, &directives, 15).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("超过限制"),
            "error should mention exceeding limit"
        );
    }

    /// P1：[check_multipart_body_size] 混入 UploadMedia 指令时仅统计 UploadMultipart
    /// 条件：directives 含 1 个 UploadMedia + 1 个 UploadMultipart（10 字节），limit = 15
    /// 断言：返回 Ok（UploadMedia 不计入 form-data 大小限制）
    #[tokio::test]
    async fn check_multipart_body_size_ignores_upload_media() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = wecom_fs::SandboxedFs::new();
        let file_a = tmp.path().join("a.txt");
        let file_b = tmp.path().join("b.txt");
        fs.atomic_write(&file_a, b"1234567890", Some(0o644))
            .await
            .unwrap();
        fs.atomic_write(&file_b, b"1234567890", Some(0o644))
            .await
            .unwrap();

        let directives = vec![
            Directive::UploadMedia {
                path: vec![PathSegment::Key("media".into())],
                file_path: file_a.clone(),
                with_file_path: false,
            },
            Directive::UploadMultipart {
                path: vec![PathSegment::Key("form".into())],
                file_path: file_b.clone(),
            },
        ];

        // UploadMultipart 文件 10 字节 < limit 15
        let result = check_multipart_body_size(&fs, &directives, 15).await;
        assert!(result.is_ok());
    }

    // ── execute_and_output：--output-dir 下载类门禁 ──

    /// 测试夹具：业务请求固定返回 `{"has_more": false}` 的后端。
    #[derive(Debug)]
    struct NoMorePagesBackend;

    impl wecom_transport::TransportBackend for NoMorePagesBackend {
        fn execute<'a>(
            &'a self,
            _endpoint: std::borrow::Cow<'a, wecom_transport::Endpoint>,
            _payload: wecom_transport::HttpRequestPayload,
            _options: wecom_transport::RequestOptions,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = std::result::Result<
                            wecom_transport::TransportResponse,
                            wecom_transport::Error,
                        >,
                    > + Send
                    + 'a,
            >,
        > {
            Box::pin(async {
                Ok(wecom_transport::TransportResponse::Json(
                    wecom_transport::ExecuteOutput {
                        result: json!({"has_more": false}),
                        extra: IndexMap::new(),
                    },
                ))
            })
        }
    }

    /// 脚本化后端的一次响应：JSON 成功、二进制响应或传输错误。
    #[derive(Debug)]
    enum ScriptedResponse {
        Json(serde_json::Value),
        Binary(wecom_transport::HttpResponse),
        Error(wecom_transport::Error),
    }

    /// 测试夹具：按调用顺序返回预设结果的可编程后端，用于驱动首页 / 次页失败。
    #[derive(Debug)]
    struct ScriptedBackend {
        responses: std::sync::Mutex<std::collections::VecDeque<ScriptedResponse>>,
    }

    impl ScriptedBackend {
        fn new(responses: Vec<ScriptedResponse>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses.into()),
            }
        }
    }

    impl wecom_transport::TransportBackend for ScriptedBackend {
        fn execute<'a>(
            &'a self,
            _endpoint: std::borrow::Cow<'a, wecom_transport::Endpoint>,
            _payload: wecom_transport::HttpRequestPayload,
            _options: wecom_transport::RequestOptions,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = std::result::Result<
                            wecom_transport::TransportResponse,
                            wecom_transport::Error,
                        >,
                    > + Send
                    + 'a,
            >,
        > {
            let response =
                self.responses
                    .lock()
                    .unwrap()
                    .pop_front()
                    .unwrap_or(ScriptedResponse::Error(wecom_transport::Error::Config(
                        "scripted backend exhausted".into(),
                    )));
            Box::pin(async move {
                match response {
                    ScriptedResponse::Json(result) => Ok(wecom_transport::TransportResponse::Json(
                        wecom_transport::ExecuteOutput {
                            result,
                            extra: IndexMap::new(),
                        },
                    )),
                    ScriptedResponse::Binary(response) => {
                        Ok(wecom_transport::TransportResponse::Binary(response))
                    }
                    ScriptedResponse::Error(e) => Err(e),
                }
            })
        }
    }

    /// 在 `root/cache` 播种 `svc.department.list`（纯 JSON 响应）的 discovery
    /// 缓存，使 `client.method(...)` 不走网络。
    #[allow(clippy::disallowed_methods)] // 测试夹具直接落盘播种缓存
    fn seed_discovery_cache(root: &std::path::Path) {
        let cache_dir = root.join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("catalog.json"),
            serde_json::to_string(&json!({ "items": [{ "name": "svc" }] })).unwrap(),
        )
        .unwrap();
        std::fs::write(
            cache_dir.join("service_svc.json"),
            serde_json::to_string(&json!({
                "description": "test service",
                "base_url": "https://test.example.com/",
                "schemas": {
                    "ListRes": { "type": "object" }
                },
                "methods": {},
                "resources": {
                    "department": {
                        "methods": {
                            "list": {
                                "http_method": "GET",
                                "path": "/list",
                                "response": { "$ref": "ListRes" }
                            }
                        },
                        "resources": {}
                    }
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    /// 在 `root/cache` 播种 `svc.department.send`（请求带 `x-wecom-file-upload`
    /// 指令）的 discovery 缓存，使 `client.method(...)` 不走网络。
    #[allow(clippy::disallowed_methods)] // 测试夹具直接落盘播种缓存
    fn seed_upload_discovery_cache(root: &std::path::Path) {
        let cache_dir = root.join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("catalog.json"),
            serde_json::to_string(&json!({ "items": [{ "name": "svc" }] })).unwrap(),
        )
        .unwrap();
        std::fs::write(
            cache_dir.join("service_svc.json"),
            serde_json::to_string(&json!({
                "description": "test service",
                "base_url": "https://test.example.com/",
                "schemas": {
                    "SendReq": {
                        "type": "object",
                        "properties": {
                            "file": { "type": "string", "x-wecom-file-upload": true }
                        }
                    },
                    "SendRes": { "type": "object" }
                },
                "methods": {},
                "resources": {
                    "department": {
                        "methods": {
                            "send": {
                                "http_method": "POST",
                                "path": "/send",
                                "request": { "$ref": "SendReq" },
                                "response": { "$ref": "SendRes" }
                            }
                        },
                        "resources": {}
                    }
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    /// P0：[execute_and_output] 非下载类方法携带 --output-dir：JSON 响应不产文件，
    ///     但显式声明仍走 WriteDir 沙箱校验
    /// 条件：方法响应为纯 JSON（无下载产物），output_dir 指向 roots 内/外两种取值
    /// 断言：roots 内 → 调用成功且目录下不产生文件；roots 外 → 沙箱报错、请求不发出
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试夹具直接落盘播种缓存，与被测 Fs 实现无关
    async fn output_dir_validated_for_non_download_method() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        // 播种 catalog + detail 缓存，使 discovery 不走网络。
        seed_discovery_cache(&root);

        let client = crate::Client::builder()
            .config_dir(&root)
            .workspace_fs(std::sync::Arc::new(wecom_fs::SandboxedFs::confined_to(&[
                root.as_path(),
            ])))
            .transport(
                wecom_transport::TransportBuilder::new(NoMorePagesBackend)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        let method = client.method(&["svc", "department", "list"]).await.unwrap();
        let run = client.run(vec!["test".into()]);
        let out_dir = root.join("out");
        std::fs::create_dir(&out_dir).unwrap();
        let mut options = RunOptions::new(&run);
        options.output_dir = Some(out_dir.clone());

        // roots 内：调用成功；纯 JSON 响应不产生文件，--output-dir 自然无效果。
        execute_and_output(&method, options)
            .await
            .expect("in-roots --output-dir must pass sandbox validation");
        let produced = std::fs::read_dir(&out_dir).unwrap().count();
        assert_eq!(produced, 0, "no file may be produced under --output-dir");

        // roots 外：显式声明的 --output-dir 走 WriteDir 沙箱校验，报错且请求不发出。
        let method = client.method(&["svc", "department", "list"]).await.unwrap();
        let run = client.run(vec!["test".into()]);
        let mut options = RunOptions::new(&run);
        options.output_dir = Some(root.join("..").to_path_buf());

        let err = execute_and_output(&method, options)
            .await
            .expect_err("out-of-roots --output-dir must be rejected by the sandbox");
        assert!(
            err.render().contains("目标路径超出可访问范围"),
            "err = {}",
            err.render()
        );
    }

    // ── execute_and_output：--output 预留文件的回退（R1 / R2） ──

    /// 构造指向 roots 内、以 `[root]` 为唯一写 roots 的 client（脚本后端）。
    fn scripted_client(root: &std::path::Path, responses: Vec<ScriptedResponse>) -> crate::Client {
        crate::Client::builder()
            .config_dir(root)
            .workspace_fs(std::sync::Arc::new(wecom_fs::SandboxedFs::confined_to(&[
                root,
            ])))
            .transport(
                wecom_transport::TransportBuilder::new(ScriptedBackend::new(responses))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    }

    /// P0：[execute_and_output] 首页请求失败时回退 --output 预留文件（R1）
    /// 条件：--output 指向 roots 内新文件，脚本后端对首页请求返回传输错误
    /// 断言：返回 Err 且预留文件已从磁盘删除（不留空壳）
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试断言：确认空壳已被回退删除
    async fn output_rolled_back_when_first_request_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        seed_discovery_cache(&root);

        let client = scripted_client(
            &root,
            vec![ScriptedResponse::Error(wecom_transport::Error::Config(
                "first page failed".into(),
            ))],
        );
        let method = client.method(&["svc", "department", "list"]).await.unwrap();
        let run = client.run(vec!["test".into()]);
        let out = root.join("out.json");
        let mut options = RunOptions::new(&run);
        options.output_path = Some(out.clone());

        let err = execute_and_output(&method, options)
            .await
            .expect_err("a first-page failure must surface");
        assert!(
            err.render().contains("first page failed"),
            "err = {}",
            err.render()
        );
        assert!(!out.exists(), "reserved file must be rolled back");
    }

    /// P0：[execute_and_output] 首页 NDJSON 写入后分页失败保留文件（R2）
    /// 条件：--output 指向 roots 内新文件 + page_count=2；首页 has_more + next_cursor，次页传输错误
    /// 断言：返回 Err，文件存在且含首页 NDJSON 行（部分结果有效）
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试断言：读回落盘 NDJSON
    async fn ndjson_kept_after_first_page_on_later_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        seed_discovery_cache(&root);

        let client = scripted_client(
            &root,
            vec![
                ScriptedResponse::Json(json!({
                    "departments": [{"id": "1"}],
                    "has_more": true,
                    "next_cursor": "cursor_1"
                })),
                ScriptedResponse::Error(wecom_transport::Error::Config(
                    "second page failed".into(),
                )),
            ],
        );
        let method = client.method(&["svc", "department", "list"]).await.unwrap();
        let run = client.run(vec!["test".into()]);
        let out = root.join("out.ndjson");
        let mut options = RunOptions::new(&run);
        options.output_path = Some(out.clone());
        options.page_count = Some(2);

        let err = execute_and_output(&method, options)
            .await
            .expect_err("a later-page failure must surface");
        assert!(
            err.render().contains("second page failed"),
            "err = {}",
            err.render()
        );

        let content = std::fs::read_to_string(&out).unwrap();
        let lines: Vec<&str> = content.trim().lines().collect();
        assert_eq!(lines.len(), 1, "only the first page is on disk: {content}");
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_json_diff::assert_json_include!(
            actual: first,
            expected: json!({"departments": [{"id": "1"}]})
        );
    }

    /// P1：[execute_and_output] 预留后 media upload 失败同样回退（R1）
    /// 条件：payload 带存在的本地文件 + schema 标记 x-wecom-file-upload，上传请求传输错误
    /// 断言：返回 Err 且 --output 预留文件已删除（首页请求未发出）
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试夹具播种上传源文件 / 断言回退结果
    async fn output_rolled_back_when_media_upload_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        seed_upload_discovery_cache(&root);
        let upload_src = root.join("upload.bin");
        std::fs::write(&upload_src, b"data").unwrap();

        let client = scripted_client(
            &root,
            vec![ScriptedResponse::Error(wecom_transport::Error::Config(
                "upload failed".into(),
            ))],
        );
        let method = client.method(&["svc", "department", "send"]).await.unwrap();
        let run = client.run(vec!["test".into()]);
        let out = root.join("out.json");
        let mut options = RunOptions::new(&run);
        options.output_path = Some(out.clone());
        options.payload = json!({ "file": upload_src.to_string_lossy() });

        let err = execute_and_output(&method, options)
            .await
            .expect_err("a media-upload failure must surface");
        assert!(
            err.render().contains("upload failed"),
            "err = {}",
            err.render()
        );
        assert!(!out.exists(), "reserved file must be rolled back");
    }

    /// P1：[execute_and_output] 空二进制（0 字节）完整下载时 --output 预留文件被保留
    /// 条件：--output 指向 roots 内新文件，脚本后端返回 0 字节的 octet-stream 响应（流正常结束）
    /// 断言：返回 Ok，预留文件保留在盘上且为 0 字节（保存场景忠实落盘）
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试断言：确认空文件保留在盘上
    async fn output_kept_on_empty_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        seed_discovery_cache(&root);

        let body: wecom_transport::ByteStream = Box::pin(futures::stream::empty());
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/octet-stream"),
        );
        let response = wecom_transport::HttpResponse::new("http://test/file", 200, headers, body);

        let client = scripted_client(&root, vec![ScriptedResponse::Binary(response)]);
        let method = client.method(&["svc", "department", "list"]).await.unwrap();
        let run = client.run(vec!["test".into()]);
        let out = root.join("empty.bin");
        let mut options = RunOptions::new(&run);
        options.output_path = Some(out.clone());

        execute_and_output(&method, options)
            .await
            .expect("a completed empty download must be saved");
        assert!(out.exists(), "the empty file must be kept on disk");
        assert_eq!(
            std::fs::metadata(&out).unwrap().len(),
            0,
            "the saved file must be 0 bytes"
        );
    }
}
