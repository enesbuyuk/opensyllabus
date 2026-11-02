mod common;

use axum::http::StatusCode;
use common::TestApp;

async fn app_with_spa() -> TestApp {
    TestApp::with_config(|config, dir| {
        let web = dir.join("web");
        std::fs::create_dir_all(web.join("assets")).unwrap();
        std::fs::write(web.join("index.html"), "<!doctype html><div id=\"root\"></div>").unwrap();
        std::fs::write(web.join("assets/app.js"), "console.log(1)").unwrap();
        config.static_dir = Some(web);
    })
    .await
}

#[tokio::test]
async fn client_routes_get_the_spa_shell_with_200() {
    let app = app_with_spa().await;
    let mut c = app.client();
    for path in ["/", "/login", "/courses/abc", "/admin/courses/abc?tab=schedule"] {
        let r = c.get(path).await.assert(StatusCode::OK);
        assert!(String::from_utf8_lossy(&r.raw).contains("id=\"root\""), "{path} did not serve index.html");
    }
}

#[tokio::test]
async fn real_files_are_served_and_missing_files_stay_404() {
    let app = app_with_spa().await;
    let mut c = app.client();
    let r = c.get("/assets/app.js").await.assert(StatusCode::OK);
    assert_eq!(String::from_utf8_lossy(&r.raw), "console.log(1)");
    c.get("/assets/old-chunk.js").await.assert(StatusCode::NOT_FOUND);
    c.get("/missing.png").await.assert(StatusCode::NOT_FOUND);
    c.get("/api/v1/nope").await.assert(StatusCode::NOT_FOUND);
}
