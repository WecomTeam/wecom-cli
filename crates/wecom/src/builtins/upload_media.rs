use serde::Deserialize;
use wecom_transport::RequestOptions;

use crate::client::EndpointKey;
use crate::{Client, Error, Result, fs};

const MEDIA_TYPE_FILE: &str = "file";

fn infer_media_type(file_path: &str) -> &'static str {
    let Some(ext) = std::path::Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
    else {
        return MEDIA_TYPE_FILE;
    };

    match ext.as_str() {
        "bmp" | "gif" | "jpg" | "jpeg" | "png" => "image",
        "amr" => "voice",
        "mp4" => "video",
        _ => MEDIA_TYPE_FILE,
    }
}

/// Response from a media upload request.
#[derive(Debug, Deserialize)]
pub struct UploadMediaResponse {
    /// The media ID returned by the server.
    pub media_id: String,
    /// Extra fields returned by the server, captured for forward-compatibility.
    #[serde(flatten)]
    pub extra: indexmap::IndexMap<String, serde_json::Value>,
}

#[tracing::instrument(level = "info", name = "media.upload", skip_all)]
pub(crate) async fn upload_media(
    client: &Client,
    fs: &fs::Fs,
    file_path: &str,
    options: &RequestOptions,
) -> Result<UploadMediaResponse> {
    tracing::info!(%file_path, "upload_media begin");

    let media_type = infer_media_type(file_path);
    let part = fs.open_as_multipart_part(file_path).await?;
    let form = reqwest::multipart::Form::new()
        .part("media", part)
        .text("type", media_type);

    let response = client
        .transport()
        .invoke(
            client.resolve_builtin_endpoint(EndpointKey::MediaUpload),
            form,
        )
        .with_options(options.clone())
        .await?
        .into_result()
        .map_err(Error::from)
        .inspect_err(|e| tracing::warn!(error = %e, "upload_media failed"))?;

    let response = UploadMediaResponse::deserialize(&response)
        .map_err(|e| {
            Error::Transport(wecom_transport::Error::Parse {
                message: format!("Failed to deserialize 'upload_media' response: {e:#}"),
                endpoint: "utils://upload_media".into(),
                body: Box::new(response),
                source: Some(e),
            })
        })
        .inspect_err(|e| tracing::warn!(error = %e, "deserialize upload_media response failed"))?;

    tracing::info!(media_id = %response.media_id, media_type, "upload_media succeeded");
    Ok(response)
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    //! ## 模块摘要：upload_media（媒体上传 HTTP 路径单元测试）
    //!
    //! ### 关键接口
    //! - [upload_media] — HTTP multipart 上传入口
    //! - [UploadMediaResponse] — 上传结果，含 media_id / extra
    //!
    //! ### 关键分支与异常路径
    //! - HTTP 成功 → result 包含 media_id
    //! - 缺 media_id → UploadMediaResponse 反序列化失败，返回 Parse 错误
    //!
    //! ### 上下游交互
    //! - 上游：client.upload_media() 公共 API
    //! - 下游：Client（transport 分发）、Fs（文件读取）

    use std::io::Write;

    use wecom_transport::RequestOptions;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::Client;

    fn make_http_test_client(base_url: &str, tmp: &std::path::Path) -> Client {
        let home = tempfile::TempDir::new().unwrap();
        let transport = wecom_transport::HttpTransportBackend::builder()
            .base_url(base_url)
            .header_sensitive("Authorization", "Bearer test-token", true)
            .build()
            .unwrap();
        Client::builder()
            .home_dir(home.path())
            .cwd(tmp)
            .writable_dir(tmp)
            .transport(transport)
            .build()
            .unwrap()
    }

    /// P0：[upload_media] HTTP 路径成功上传
    /// 条件：wiremock mock /file/upload 端点，返回 media_id
    /// 断言：multipart type 按扩展名推断为 image，result.media_id == "HTTP_MEDIA_001"
    #[tokio::test]
    async fn upload_media_http_success() {
        let server = MockServer::start().await;
        let tmp = tempfile::tempdir().unwrap();
        let test_file = tmp.path().join("photo.jpg");
        let mut f = std::fs::File::create(&test_file).unwrap();
        f.write_all(b"fake-jpeg-bytes").unwrap();
        f.flush().unwrap();
        drop(f);
        let file_path_str = test_file.to_string_lossy().to_string();

        Mock::given(method("POST"))
            .and(path("/file/upload"))
            .and(wiremock::matchers::body_string_contains(r#"name="type""#))
            .and(wiremock::matchers::body_string_contains(
                "\r\n\r\nimage\r\n",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": r#"{"media_id":"HTTP_MEDIA_001"}"#,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = make_http_test_client(&server.uri(), tmp.path());
        let fs = client.default_fs();
        let result = upload_media(&client, &fs, &file_path_str, &RequestOptions::default())
            .await
            .expect("upload_media HTTP should succeed");

        assert_eq!(result.media_id, "HTTP_MEDIA_001");
    }

    /// P0：[infer_media_type] 图片/音频/视频扩展名映射为企业微信媒体类型
    /// 条件：传入常见图片、amr、mp4 路径
    /// 断言：分别返回 image / voice / video
    #[test]
    fn infer_media_type_known_extensions() {
        assert_eq!(infer_media_type("/tmp/example.PNG"), "image");
        assert_eq!(infer_media_type("/tmp/example.jpeg"), "image");
        assert_eq!(infer_media_type("/tmp/example.amr"), "voice");
        assert_eq!(infer_media_type("/tmp/example.mp4"), "video");
    }

    /// P1：[infer_media_type] 未识别扩展名回退为 file
    /// 条件：传入 pdf 与无扩展名路径
    /// 断言：返回 file
    #[test]
    fn infer_media_type_unknown_extensions_fallback_to_file() {
        assert_eq!(infer_media_type("/tmp/example.pdf"), "file");
        assert_eq!(infer_media_type("/tmp/example"), "file");
    }
}
