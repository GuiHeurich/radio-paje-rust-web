use actix_files::NamedFile;
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, Responder, rt, web};
use askama::Template;
use mime_guess::from_path;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use tempfile::NamedTempFile;

mod auth;
mod handler;
mod listing;

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
    log::info!("Streaming audio");

    let bucket_id = std::env::var("B2_BUCKET_ID").map_err(|e| {
        log::error!("Missing B2_BUCKET_ID: {}", e);
        actix_web::error::ErrorInternalServerError("Missing B2_BUCKET_ID")
    })?;

    log::info!("Fetched bucket {}", bucket_id);

    let auth = auth::authenticate().await.map_err(|e| {
        log::error!("Authentication failed: {}", e);
        actix_web::error::ErrorInternalServerError("Auth failed")
    })?;

    let file_name = listing::select_random_file(&auth, &bucket_id)
        .await
        .map_err(|e| {
            log::error!("Failed to select random file: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to select random file")
        })?;

    log::info!("Fetched file {}", file_name);

    let download_url = format!("{}/file/radio-paje-music/{}", auth.api_url, file_name);

    log::info!("Downloading file from {}", download_url);

    let bytes = reqwest::Client::new()
        .get(&download_url)
        // .bearer_auth(&auth.authorization_token)
        .send()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .bytes()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("Downloaded {} bytes", bytes.len());

    let mut temp_file = NamedTempFile::new().map_err(actix_web::error::ErrorInternalServerError)?;
    temp_file
        .write_all(&bytes)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("Downloaded file to {}", temp_file.path().display());
    log::info!(
        "Downloaded file size: {}",
        temp_file
            .as_file()
            .metadata()
            .map_err(actix_web::error::ErrorInternalServerError)?
            .len()
    );

    let file_path: PathBuf = temp_file.into_temp_path().keep().unwrap();
    let mime_type = from_path(&file_path).first_or_octet_stream();
    let file = NamedFile::open(file_path)?;

    Ok(file.set_content_type(mime_type))
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    log::info!("Starting server");

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
