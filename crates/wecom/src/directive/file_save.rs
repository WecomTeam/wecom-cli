use base64::Engine;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use super::types::Directive;
use crate::telemetry::contract::file_save_invalid as ctr;
use crate::{Error, Result, RunOptions, fs, json_path, telemetry, util};

/// 后端返回的 `file_save` 字段的 Object 格式。
///
/// 当值为 Object 时，`file_name` 和 `content_encoding` 覆盖同名 schema 设置，
/// `content` 为文件内容。
#[derive(Debug, Deserialize)]
struct FileSavePayload {
    #[serde(rename = "file_name")]
    file_name: Option<String>,
    content: String,
    #[serde(rename = "content_encoding")]
    content_encoding: Option<String>,
}

/// 从 JSON 响应中提取标记了 `x-wecom-file-save` 的字段值，保存为独立文件，
/// 并将 JSON 中对应的值替换为文件路径。
///
/// 写入或 sync 失败时先回退（best-effort 删除）该字段新建的文件，再返回错误：
/// 半成品不算有效产物，也不残留指向它的 JSON 路径。
#[tracing::instrument(
    level = "debug",
    name = "directive.file_save",
    skip_all,
    fields(file_name = tracing::field::Empty),
)]
pub async fn process_file_save(
    options: &RunOptions<'_>,
    data: &mut serde_json::Value,
    directive: &Directive<'_>,
) -> Result<()> {
    let fs = options.run.get_fs();
    let Directive::Save {
        path,
        options: save_options,
    } = directive
    else {
        return Ok(());
    };

    // 从 result 中按 path 取出对应的字符串值或对象
    let Some(raw_value) = json_path::get_value_deep(data, path) else {
        return Ok(());
    };

    // file_save 字段值支持 String 或 Object 两种格式：
    // - String: 直接作为文件内容
    // - Object: { file_name?, content, content_encoding? }，其中 file_name 和 content_encoding 覆盖 schema 设置
    let payload = if raw_value.is_object() {
        match serde_json::from_value(raw_value.clone()) {
            Ok(payload) => payload,
            Err(e) => {
                let err_msg = e.to_string();
                tracing::warn!(error = %e, "Invalid file_save object");
                telemetry::emit(
                    ctr::KIND,
                    &serde_json::json!({
                        ctr::FIELD_OUTCOME: ctr::OUTCOME_INVALID_OBJECT,
                        ctr::FIELD_ERROR: &err_msg,
                    }),
                );
                return Ok(());
            }
        }
    } else if let Some(s) = raw_value.as_str() {
        FileSavePayload {
            file_name: None,
            content: s.to_string(),
            content_encoding: None,
        }
    } else {
        tracing::warn!(value = %raw_value, "Invalid file_save value");
        telemetry::emit(
            ctr::KIND,
            &serde_json::json!({
                ctr::FIELD_OUTCOME: ctr::OUTCOME_INVALID_TYPE,
            }),
        );
        return Ok(());
    };

    // Object 字段覆盖同名 schema 设置
    let content_encoding = payload
        .content_encoding
        .as_deref()
        .or(save_options.content_encoding.as_deref());

    let file_bytes = decode_content(&payload.content, content_encoding)
        .inspect_err(|e| tracing::warn!(error = %e, "decode file save content failed"))?;

    let target = options.output_dir();

    let file_name = payload
        .file_name
        .or_else(|| save_options.file_name.clone())
        .map_or_else(|| util::random_str(32), |s| fs::sanitize_filename(&s));

    let (output_path, mut file) = fs
        .create_file_unique(&target.join(file_name))
        .await
        .inspect_err(|e| tracing::warn!(error = %e, "create output file failed"))?;

    if let Err(e) = async {
        file.write_all(&file_bytes)
            .await
            .map_err(|e| Error::io(format!("Failed to write to {}", output_path.display()), e))
            .inspect_err(|e| tracing::warn!(error = %e, "write file failed"))?;

        file.sync_all()
            .await
            .inspect_err(|e| tracing::warn!(error = %e, "sync file failed"))?;

        Ok::<(), Error>(())
    }
    .await
    {
        if let Err(error) = file.discard().await {
            tracing::warn!(%error, path = %output_path.display(), "rollback saved file failed");
        }
        return Err(e);
    }

    tracing::info!(path = %output_path.display(), size = file_bytes.len(), "Saved extra file");

    // 将 JSON 中对应的值替换为文件路径
    json_path::set_value_deep(
        data,
        path,
        serde_json::Value::String(output_path.to_string_lossy().to_string()),
    );

    Ok(())
}

/// 解码文件内容：如果标记了 base64 则解码，否则直接使用原始字符串。
fn decode_content(content_str: &str, content_encoding: Option<&str>) -> Result<Vec<u8>> {
    if content_encoding != Some("base64") {
        return Ok(content_str.as_bytes().to_vec());
    }
    base64::engine::general_purpose::STANDARD
        .decode(content_str)
        .map_err(|e| Error::other(format!("Decode base64 failed: {e:#}").into()))
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    //! ## 模块摘要：file_save（文件保存指令处理）
    //!
    //! ### 关键接口
    //! - [process_file_save] — 从 JSON 中提取标记字段，保存为文件并替换值为路径
    //! - [decode_content] — 解码文件内容（base64 或原始字节）
    //!
    //! ### 关键分支与异常路径
    //! - 非 Save 指令 → 空操作返回 Ok
    //! - JSON 路径不存在或值非字符串 → 空操作
    //! - content_encoding 为 "base64" → base64 解码；其他值 → 原始字节
    //! - 非法 base64 → 返回 Other Error
    //! - decode / create 失败 → Err 经 ? 传播
    //! - write / sync 失败 → 回退删除该字段的文件后再返回 Err（不残留半成品）
    //!
    //! ### 上下游交互
    //! - 上游：HTTP 响应处理后调用 process_file_save 执行文件保存指令
    //! - 下游：显式输出目录经 `run.get_fs()`，缺省请求目录经内部 Fs；经 [json_path] 读写 JSON 路径

    use std::fs;

    use super::*;
    use crate::Client;
    use crate::fs::testing::FailingWriterFs;

    /// Build a minimal [Client] anchored at [dir] ([cwd] = workspace root),
    /// with [config_dir] in a separate subdirectory — the default workspace_fs
    /// denies config_dir, so the two must not coincide.
    fn test_client(dir: &std::path::Path) -> Client {
        Client::builder()
            .cwd(dir)
            .config_dir(dir.join("config"))
            .build()
            .unwrap()
    }

    /// Build [RunOptions] from a [Client] with output_dir set to [dir].
    fn test_options<'r>(
        run: &'r crate::client::CliRun<'r>,
        dir: &std::path::Path,
    ) -> RunOptions<'r> {
        let mut opts = RunOptions::new(run);
        opts.output_dir = Some(dir.to_path_buf());
        opts
    }

    // ── decode_content ──

    /// P0：无编码参数时直接返回原始字节
    /// 条件：content_encoding 为 None
    /// 断言：返回原始字符串的字节表示
    #[test]
    fn decode_content_no_encoding() {
        let result = decode_content("hello world", None).unwrap();
        assert_eq!(result, b"hello world");
    }

    /// P1：非 base64 编码类型时返回原始字节
    /// 条件：content_encoding 为 "utf-8"（非 base64）
    /// 断言：返回原始字符串的字节表示，不进行解码
    #[test]
    fn decode_content_non_base64_encoding() {
        let result = decode_content("hello", Some("utf-8")).unwrap();
        assert_eq!(result, b"hello");
    }

    /// P0：合法 base64 编码内容正确解码
    /// 条件：content_encoding 为 "base64"，输入为 "hello" 的 base64 编码
    /// 断言：返回解码后的字节 b"hello"
    #[test]
    fn decode_content_base64_valid() {
        // "hello" in base64 = "aGVsbG8="
        let result = decode_content("aGVsbG8=", Some("base64")).unwrap();
        assert_eq!(result, b"hello");
    }

    /// P1：非法 base64 内容返回错误
    /// 条件：content_encoding 为 "base64"，输入为非 base64 字符串
    /// 断言：返回 Err
    #[test]
    fn decode_content_base64_invalid() {
        let result = decode_content("not_valid_base64!!!", Some("base64"));
        assert!(result.is_err());
    }

    /// P1：空字符串的 base64 解码
    /// 条件：content_encoding 为 "base64"，输入为空字符串
    /// 断言：返回空字节向量
    #[test]
    fn decode_content_base64_empty() {
        let result = decode_content("", Some("base64")).unwrap();
        assert!(result.is_empty());
    }

    // ── process_file_save ──

    /// P1：非 Save 指令为空操作
    /// 条件：传入 UploadMedia 指令而非 Save 指令
    /// 断言：函数正常返回 Ok，数据不变
    #[tokio::test]
    async fn process_file_save_non_save_directive_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());
        let mut data = serde_json::json!({"file": "/tmp/test"});
        let directive = Directive::UploadMedia {
            path: vec![],
            file_path: std::path::PathBuf::from("/tmp/test"),
            with_file_path: false,
        };
        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());
    }

    /// P0：Save 指令正确写入文件并替换 JSON 值
    /// 条件：Save 指令指定文件名和纯文本内容
    /// 断言：JSON 值被替换为文件路径，文件内容与原始值一致
    #[tokio::test]
    async fn process_file_save_writes_content_and_replaces_value() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("output.txt".to_string()),
            content_encoding: None,
        };

        let mut data = serde_json::json!({"content": "file content here"});
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("content".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());

        // The JSON value should now be a file path
        let replaced = data["content"].as_str().unwrap();
        assert!(replaced.contains("output"));

        // The file should actually exist with the content
        let content = fs::read_to_string(replaced).unwrap();
        assert_eq!(content, "file content here");
    }

    /// P0：Save 指令正确处理 base64 编码内容
    /// 条件：Save 指令指定 content_encoding 为 base64
    /// 断言：文件内容为解码后的原始字节
    #[tokio::test]
    async fn process_file_save_base64_content() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("decoded.bin".to_string()),
            content_encoding: Some("base64".to_string()),
        };

        // "hello" in base64
        let mut data = serde_json::json!({"data": "aGVsbG8="});
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());

        let path = data["data"].as_str().unwrap();
        let content = fs::read(path).unwrap();
        assert_eq!(content, b"hello");
    }

    /// P1：JSON 路径不存在时 Save 指令为空操作
    /// 条件：Save 指定的路径在 JSON 数据中不存在
    /// 断言：函数正常返回 Ok，原始数据保持不变
    #[tokio::test]
    async fn process_file_save_missing_path_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("out.txt".to_string()),
            content_encoding: None,
        };

        let mut data = serde_json::json!({"other": "value"});
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key(
                "nonexistent".to_string(),
            )],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());
        // data should be unchanged
        assert_eq!(data["other"], "value");
    }

    // ── 错误传播 ──

    /// P1：[process_file_save] base64 解码失败时返回错误
    /// 条件：Save 指令 content 为非法 base64，content_encoding = "base64"
    /// 断言：返回 Err（decode 失败经 ? 传播），JSON 值未被替换为路径
    #[tokio::test]
    async fn process_file_save_invalid_base64_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("bad.bin".to_string()),
            content_encoding: Some("base64".to_string()),
        };
        let mut data = serde_json::json!({"data": "not_valid_base64!!!"});
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_err());
        assert_eq!(data["data"], "not_valid_base64!!!");
    }

    /// P1：[process_file_save] 底层 Fs 创建文件失败时返回错误
    /// 条件：workspace_fs 注入 ErrFs（全部操作一律 Permission 拒绝）
    /// 断言：返回 Err（create_file_unique 失败经 ? 传播）
    #[tokio::test]
    async fn process_file_save_create_failure_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let client = Client::builder()
            .workspace_fs(std::sync::Arc::new(crate::fs::testing::ErrFs))
            .config_dir(dir.path())
            .build()
            .unwrap();
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("out.txt".to_string()),
            content_encoding: None,
        };
        let mut data = serde_json::json!({"data": "content"});
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_err());
    }

    /// P1：[process_file_save] 写入内容失败时返回错误且回退半成品
    /// 条件：Fs 桩的 writer 在 write_all 时注入失败，create_file 已真实落盘
    /// 断言：返回 Err，且创建过的文件已被回退删除
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试断言：确认半成品已被回退删除
    async fn process_file_save_write_failure_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let stub = FailingWriterFs::new(true);
        let client = stub.clone().client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("out.txt".to_string()),
            content_encoding: None,
        };
        let mut data = serde_json::json!({"data": "content"});
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_err());

        let created = stub.created_paths();
        assert!(!created.is_empty(), "create_file must have been called");
        for path in created {
            assert!(!path.exists(), "partial file must be rolled back: {path:?}");
        }
    }

    /// P1：[process_file_save] sync 失败时返回错误且回退半成品
    /// 条件：Fs 桩的 writer 写入成功但 sync_all 注入失败
    /// 断言：返回 Err，且创建过的文件已被回退删除
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // 测试断言：确认半成品已被回退删除
    async fn process_file_save_sync_failure_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let stub = FailingWriterFs::new(false);
        let client = stub.clone().client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("out.txt".to_string()),
            content_encoding: None,
        };
        let mut data = serde_json::json!({"data": "content"});
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_err());

        let created = stub.created_paths();
        assert!(!created.is_empty(), "create_file must have been called");
        for path in created {
            assert!(!path.exists(), "partial file must be rolled back: {path:?}");
        }
    }

    // ── file_save Object format ──

    /// P0：[process_file_save] Object 格式含全部字段，覆盖 schema 设置
    /// 条件：file_save 值为 Object { file_name, content, content_encoding }，schema 设置不同的 file_name
    /// 断言：使用 Object 中的 file_name 和 content_encoding 覆盖 schema，content 正确落盘后 JSON 值被替换为路径
    #[tokio::test]
    async fn process_file_save_object_all_fields_overrides_schema() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        // Schema sets file_name "schema_name.txt" and encoding "utf-8"
        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("schema_name.txt".to_string()),
            content_encoding: Some("utf-8".to_string()),
        };

        // Object overrides with different file_name and base64 encoding
        let mut data = serde_json::json!({
            "data": {
                "file_name": "override_name.bin",
                "content": "aGVsbG8=",
                "content_encoding": "base64"
            }
        });
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());

        // JSON value replaced with file path
        let path = data["data"].as_str().unwrap();
        assert!(path.contains("override_name"));

        // File content should be base64-decoded "hello"
        let content = fs::read(path).unwrap();
        assert_eq!(content, b"hello");
    }

    /// P0：[process_file_save] Object 格式仅含 content，file_name 回退到 schema
    /// 条件：file_save 值为 Object { content: "..." }，schema 设置了 file_name
    /// 断言：使用 schema 的 file_name，content 正确落盘
    #[tokio::test]
    async fn process_file_save_object_content_only_falls_back_to_schema() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("schema_output.txt".to_string()),
            content_encoding: None,
        };

        let mut data = serde_json::json!({
            "data": {
                "content": "file content from object"
            }
        });
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());

        let path = data["data"].as_str().unwrap();
        assert!(path.contains("schema_output"));

        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "file content from object");
    }

    /// P1：[process_file_save] Object 格式 file_name 覆盖 schema 的 file_name
    /// 条件：file_save 值为 Object { file_name: "obj.txt", content: "..." }
    /// 断言：文件名为 Object 中的 "obj.txt"
    #[tokio::test]
    async fn process_file_save_object_file_name_override() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("schema.txt".to_string()),
            content_encoding: None,
        };

        let mut data = serde_json::json!({
            "data": {
                "file_name": "obj_name.txt",
                "content": "hello obj"
            }
        });
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());

        let path = data["data"].as_str().unwrap();
        assert!(path.contains("obj_name"));

        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "hello obj");
    }

    /// P1：[process_file_save] Object 格式 content_encoding 覆盖 schema 的编码设置
    /// 条件：file_save 值为 Object { content: base64_str, content_encoding: "base64" }
    /// 断言：文件内容为 base64 解码后的字节
    #[tokio::test]
    async fn process_file_save_object_content_encoding_override() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        // Schema has no encoding
        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("decoded.bin".to_string()),
            content_encoding: None,
        };

        // Object overrides encoding to base64
        let mut data = serde_json::json!({
            "data": {
                "content": "aGVsbG8=",
                "content_encoding": "base64"
            }
        });
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());

        let path = data["data"].as_str().unwrap();
        let content = fs::read(path).unwrap();
        assert_eq!(content, b"hello");
    }

    /// P0：[process_file_save] 响应体中的 file_name 经 sanitize 净化为单段
    /// 条件：file_save 值为 Object { file_name: "../../evil.txt", content }
    /// 断言：落盘文件直接位于 output_dir 下，文件名为净化后的单段，不产生越界
    #[tokio::test]
    async fn process_file_save_sanitizes_payload_file_name() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: None,
            content_encoding: None,
        };
        let mut data = serde_json::json!({
            "data": {
                "file_name": "../../evil.txt",
                "content": "payload"
            }
        });
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());

        let path = std::path::PathBuf::from(data["data"].as_str().unwrap());
        // Canonicalize both sides: the fs layer returns resolved real paths
        // (on macOS the tempdir /var base is symlinked to /private/var).
        let expected_dir = dir.path().canonicalize().unwrap();
        let actual_dir = path.parent().map(|p| p.canonicalize().unwrap());
        assert_eq!(
            actual_dir.as_deref(),
            Some(expected_dir.as_path()),
            "sanitized file must land directly under output_dir: {path:?}"
        );
        let name = path.file_name().unwrap().to_string_lossy();
        assert_eq!(name, ".._.._evil.txt", "name = {name}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "payload");
    }

    /// P1：[process_file_save] 无 file_name 时使用 random_str 自生成名单段落盘
    /// 条件：file_save 值为纯字符串，schema 与 payload 均未提供 file_name
    /// 断言：落盘文件直接位于 output_dir 下，文件名为 32 位随机字母数字
    ///      （random_str 自生成安全单段，无需 sanitize —— 豁免锁定）
    #[tokio::test]
    async fn process_file_save_random_name_exempt_from_sanitize() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: None,
            content_encoding: None,
        };
        let mut data = serde_json::json!({"content": "random-name-content"});
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("content".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());

        let path = std::path::PathBuf::from(data["content"].as_str().unwrap());
        // Canonicalize both sides: the fs layer returns resolved real paths
        // (on macOS the tempdir /var base is symlinked to /private/var).
        let expected_dir = dir.path().canonicalize().unwrap();
        let actual_dir = path.parent().map(|p| p.canonicalize().unwrap());
        assert_eq!(
            actual_dir.as_deref(),
            Some(expected_dir.as_path()),
            "file must land directly under output_dir: {path:?}"
        );
        let name = path.file_name().unwrap().to_string_lossy();
        assert_eq!(name.len(), 32, "expected 32-char random name, got {name}");
        assert!(
            name.chars().all(|c| c.is_ascii_alphanumeric()),
            "expected alphanumeric name, got {name}"
        );
    }

    /// P1：[process_file_save] Object 格式无 content 字段返回空操作
    /// 条件：file_save 值为 Object { file_name: "x.txt" } 但缺少 content 字段
    /// 断言：不做任何操作，原始对象值保持不变
    #[tokio::test]
    async fn process_file_save_object_missing_content_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let client = test_client(dir.path());
        let run = client.run(vec!["test".into()]);
        let options = test_options(&run, dir.path());

        let save_options = crate::schema::FileSaveOptions {
            file_name: Some("out.txt".to_string()),
            content_encoding: None,
        };

        let mut data = serde_json::json!({
            "data": {
                "file_name": "x.txt"
            }
        });
        let original = data.clone();
        let directive = Directive::Save {
            path: vec![crate::json_path::PathSegment::Key("data".to_string())],
            options: &save_options,
        };

        let result = process_file_save(&options, &mut data, &directive).await;
        assert!(result.is_ok());
        // data 应保持不变
        assert_eq!(data, original);
    }
}
