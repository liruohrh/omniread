use mockito::mock;
use rust_lib_omniread::http_client::HttpClient;
use rust_lib_omniread::request_config::{RequestConfig, RequestMethod};
use serde_json::json;

#[tokio::test]
async fn test_http_client_get() {
    let _m = mock("GET", "/test")
        .with_status(200)
        .with_body("Hello, World!")
        .create();

    let client = HttpClient::new().unwrap();
    let result = client.get(&format!("{}/test", mockito::server_url())).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, World!");
}

#[tokio::test]
async fn test_http_client_post_with_body() {
    let _m = mock("POST", "/api/data")
        .with_status(200)
        .with_body(r#"{"status": "ok"}"#)
        .match_body(mockito::Matcher::Json(json!({"key": "value"})))
        .create();

    let mut config = RequestConfig::new(format!("{}/api/data", mockito::server_url()));
    config.method = RequestMethod::Post;
    config.content_type = Some("application/json".to_string());
    config.body = Some(json!({"key": "value"}));

    let client = HttpClient::new().unwrap();
    let result = client.execute(config).await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), r#"{"status": "ok"}"#);
}

#[tokio::test]
async fn test_http_client_custom_headers() {
    let _m = mock("GET", "/headers")
        .with_status(200)
        .with_body("ok")
        .match_header("X-Custom", "test-value")
        .create();

    let mut config = RequestConfig::new(format!("{}/headers", mockito::server_url()));
    config.headers.insert("X-Custom".to_string(), "test-value".to_string());

    let client = HttpClient::new().unwrap();
    let result = client.execute(config).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_http_client_header_priority() {
    let _m = mock("GET", "/priority")
        .with_status(200)
        .with_body("ok")
        .match_header("X-Override", "custom-value")
        .match_header("X-Default", "default-value")
        .create();

    let mut client = HttpClient::new().unwrap();
    client.set_default_header("X-Default".to_string(), "default-value".to_string());

    let mut config = RequestConfig::new(format!("{}/priority", mockito::server_url()));
    config.headers.insert("X-Override".to_string(), "custom-value".to_string());

    let result = client.execute(config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_http_client_different_methods() {
    let methods = vec![
        (RequestMethod::Get, "GET"),
        (RequestMethod::Post, "POST"),
        (RequestMethod::Put, "PUT"),
        (RequestMethod::Delete, "DELETE"),
        (RequestMethod::Patch, "PATCH"),
    ];

    let client = HttpClient::new().unwrap();

    for (method, method_str) in methods {
        let _m = mock(method_str, "/method-test")
            .with_status(200)
            .with_body(method_str)
            .create();

        let mut config = RequestConfig::new(format!("{}/method-test", mockito::server_url()));
        config.method = method;

        let result = client.execute(config).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), method_str);
    }
}

#[tokio::test]
async fn test_http_client_error_status() {
    let _m = mock("GET", "/error")
        .with_status(404)
        .create();

    let client = HttpClient::new().unwrap();
    let result = client.get(&format!("{}/error", mockito::server_url())).await;
    
    assert!(result.is_err());
}
