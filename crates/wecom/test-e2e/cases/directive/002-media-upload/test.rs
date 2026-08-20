fn media_upload_service_body(service_base_url: &str) -> serde_json::Value {
    api_response(&json!({
        "description": "Message service with media upload",
        "base_url": service_base_url,
        "schemas": {
            "MsgSendReq": {
                "type": "object",
                "properties": {
                    "media_id": {
                        "type": "string",
                        "x-wecom-file-upload": true
                    },
                    "content": { "type": "string" }
                }
            },
            "MsgSendRes": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" }
                }
            }
        },
        "methods": {},
        "resources": {
            "msg": {
                "methods": {
                    "send": {
                        "path": "/msg/send",
                        "http_method": "POST",
                        "request": { "$ref": "MsgSendReq" },
                        "response": { "$ref": "MsgSendRes" }
                    }
                },
                "resources": {}
            }
        }
    }))
}

#[tokio::test]
async fn run() {
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    // Discovery: catalog + service detail with x-wecom-file-upload directive
    let catalog_body = api_response(&json!({
        "items": [{ "name": "msgsvc", "description": "Message service" }]
    }));
    Mock::given(method("POST"))
        .and(path("/service/discovery"))
        .respond_with(ResponseTemplate::new(200).set_body_json(catalog_body))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/service/discovery"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(media_upload_service_body(&server.uri())),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    // Media upload endpoint: verify multipart body contains the file content
    Mock::given(method("POST"))
        .and(path("/file/upload"))
        .and(body_string_contains(r#"name="media""#))
        .and(body_string_contains("fake-jpeg-content"))
        .and(body_string_contains(r#"name="type""#))
        .and(body_string_contains("\r\n\r\nimage\r\n"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(api_response(&json!({
                "media_id": "MEDIA_001"
            }))),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Method call endpoint
    Mock::given(method("POST"))
        .and(path("/msg/send"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(api_response(&json!({
                "ok": true
            }))),
        )
        .expect(1)
        .mount(&server)
        .await;

    let tmp = tempfile::tempdir().unwrap();
    let photo_path = tmp.path().join("photo.jpg");
    #[allow(clippy::disallowed_methods)]
    std::fs::write(&photo_path, b"fake-jpeg-content").unwrap();

    let buf = SharedBuf::new();
    let client = wecom::Client::builder()
        .home_dir(tmp.path())
        .tmp_dir(tmp.path())
        .transport(
            wecom::transport::HttpTransportBackend::builder()
                .base_url(server.uri())
                .header_sensitive("Authorization", "Bearer test-token", true)
                .build()
                .expect("add header"),
        )
        .cwd(tmp.path().to_path_buf())
        .readable_dirs(vec![tmp.path().to_path_buf()])
        .build()
        .unwrap();

    let result = client
        .run(vec![
            "wecom".into(),
            "msgsvc".into(),
            "msg".into(),
            "send".into(),
            "--media-id".into(),
            photo_path.to_str().unwrap().into(),
            "--content".into(),
            "hello".into(),
        ])
        .output(wecom::CliRunOutput::new(buf.clone()))
        .await;
    assert_cli_ok(&result, &buf, "media upload e2e");

    let v = assert_stdout_json(&buf);
    assert_eq!(v["ok"], true);
}
