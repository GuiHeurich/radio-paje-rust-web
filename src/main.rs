use actix_files::NamedFile;
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, Responder, rt, web};
use askama::Template;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use tempfile::NamedTempFile;

mod handler;

#[derive(Template)]
#[template(path = "index.html")]
struct Index;

async fn index(
    _query: web::Query<HashMap<String, String>>,
) -> Result<impl Responder, actix_web::Error> {
    let html = Index.render().expect("template should be valid");

    Ok(web::Html::new(html))
}

/// Handshake and start WebSocket handler with heartbeats.
async fn echo_heartbeat_ws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    // spawn websocket handler (and don't await it) so that the response is returned immediately
    rt::spawn(handler::echo_heartbeat_ws(session, msg_stream));

    Ok(res)
}

async fn stream_audio() -> Result<NamedFile, Error> {
    let url = "https://f005.backblazeb2.com/file/radio-paje-music/um.opus";
    let response = reqwest::get(url)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let bytes = response
        .bytes()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let mut temp_file = NamedTempFile::new().map_err(actix_web::error::ErrorInternalServerError)?;
    temp_file
        .write_all(&bytes)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let file_path: PathBuf = temp_file.path().to_path_buf();
    let file = NamedFile::open(file_path)?;
    Ok(file.set_content_type(mime::Mime::from_str("audio/opus").unwrap()))
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::resource("/").route(web::get().to(index)))
            .service(web::resource("/echo").route(web::get().to(echo_heartbeat_ws)))
            .service(web::resource("/stream").route(web::get().to(stream_audio)))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test, web};

    #[actix_web::test]
    async fn test_index_returns_html() {
        let mut app =
            test::init_service(App::new().service(web::resource("/").route(web::get().to(index))))
                .await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;

        let body_str = String::from_utf8_lossy(&body);
        assert!(body_str.contains("<html") || body_str.contains("<!DOCTYPE html"));
    }
}
