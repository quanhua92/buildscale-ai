use crate::common::TestApp;

#[tokio::test]
async fn test_health_endpoint_returns_200() {
    let app = TestApp::new().await;

    let response = app.client
        .get(&app.url("/api/v1/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_health_endpoint_returns_json() {
    let app = TestApp::new().await;

    let response = app.client
        .get(&app.url("/api/v1/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_health_endpoint_returns_cache_metrics() {
    let app = TestApp::new().await;

    let response = app.client
        .get(&app.url("/api/v1/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let metrics: serde_json::Value = response.json().await.unwrap();

    // Verify all fields exist
    assert!(metrics.get("num_keys").is_some());
    assert!(metrics.get("last_worker_time").is_some());
    assert!(metrics.get("cleaned_count").is_some());
    assert!(metrics.get("size_bytes").is_some());
}

#[tokio::test]
async fn test_health_endpoint_with_cache_data() {
    let app = TestApp::new().await;

    // Add some data to cache
    app.cache.set("test_key", "test_value".to_string()).await.unwrap();
    app.cache.set("another_key", "another_value".to_string()).await.unwrap();

    let response = app.client
        .get(&app.url("/api/v1/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let metrics: serde_json::Value = response.json().await.unwrap();

    // Should have 2 keys
    assert_eq!(metrics["num_keys"], 2);
}

#[tokio::test]
async fn test_multiple_apps_can_run_concurrently() {
    // Create multiple test apps to verify port randomization works
    let app1 = TestApp::new().await;
    let app2 = TestApp::new().await;
    let app3 = TestApp::new().await;

    // All should have different addresses
    assert_ne!(app1.address, app2.address);
    assert_ne!(app2.address, app3.address);
    assert_ne!(app1.address, app3.address);

    // All should work
    let response1 = app1.client.get(&app1.url("/api/v1/health")).send().await.unwrap();
    let response2 = app2.client.get(&app2.url("/api/v1/health")).send().await.unwrap();
    let response3 = app3.client.get(&app3.url("/api/v1/health")).send().await.unwrap();

    assert_eq!(response1.status(), 200);
    assert_eq!(response2.status(), 200);
    assert_eq!(response3.status(), 200);
}
