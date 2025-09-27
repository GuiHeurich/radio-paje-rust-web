use std::{
    collections::HashMap,
};
use actix_web::{web, App, HttpServer, Responder};
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
struct Index;

async fn index(_query: web::Query<HashMap<String, String>>) -> Result<impl Responder, actix_web::Error> {
    let html = Index.render().expect("template should be valid");

    Ok(web::Html::new(html))
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::resource("/").route(web::get().to(index)))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};

    #[actix_web::test]
    async fn test_index_returns_html() {
        let mut app = test::init_service(
            App::new().service(web::resource("/").route(web::get().to(index)))
        ).await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;

        let body_str = String::from_utf8_lossy(&body);
        assert!(body_str.contains("<html") || body_str.contains("<!DOCTYPE html"));
    }
}
