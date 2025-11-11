use actix_files::NamedFile;
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, Responder, rt, web};
use askama::Template;
use mime_guess::from_path;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod auth;
mod handler;
mod listing;

#[derive(Template)]
#[template(path = "index.html")]
struct Index;

#[derive(Clone, Debug)]
struct PlaybackState {
    is_playing: bool,
    queue: VecDeque<String>,
}

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

async fn stream_audio(
    req: HttpRequest,
    query: web::Query<HashMap<String, String>>,
    _playback: web::Data<Arc<Mutex<PlaybackState>>>,
) -> Result<HttpResponse, Error> {
    let bucket_id = std::env::var("B2_BUCKET_ID").map_err(|e| {
        log::error!("Missing B2_BUCKET_ID: {}", e);
        actix_web::error::ErrorInternalServerError("Missing B2_BUCKET_ID")
    })?;

    // Check if a specific file is requested via query parameter
    let file_name = if let Some(requested_file) = query.get("file") {
        log::info!("Serving requested file: {}", requested_file);
        requested_file.clone()
    } else {
        // Only select a random file if no file is specified
        log::info!("Fetched bucket {}", bucket_id);

        let auth = auth::authenticate().await.map_err(|e| {
            log::error!("Authentication failed: {}", e);
            actix_web::error::ErrorInternalServerError("Auth failed")
        })?;

        let random_file = listing::select_random_file(&auth, &bucket_id)
            .await
            .map_err(|e| {
                log::error!("Failed to select random file: {}", e);
                actix_web::error::ErrorInternalServerError("Failed to select random file")
            })?;

        log::info!("Selected random file: {}", random_file);

        // Redirect to the same endpoint with the file parameter
        // URL encode the filename to handle special characters
        let encoded_file = random_file
            .replace("%", "%25")
            .replace(" ", "%20")
            .replace("?", "%3F")
            .replace("#", "%23");

        return Ok(HttpResponse::Found()
            .append_header(("Location", format!("/stream?file={}", encoded_file)))
            .finish());
    };

    log::info!("Fetching file: {}", file_name);

    // Check cache first
    let cache_dir = PathBuf::from("/tmp/radio-paje-cache");
    fs::create_dir_all(&cache_dir)?;

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    file_name.hash(&mut hasher);
    let hash = hasher.finish();
    let unique_name = format!("{}_{}", hash, file_name);
    let file_path = cache_dir.join(&unique_name);

    // Download if not cached
    if !file_path.exists() {
        let auth = auth::authenticate().await.map_err(|e| {
            log::error!("Authentication failed: {}", e);
            actix_web::error::ErrorInternalServerError("Auth failed")
        })?;

        let download_url = format!("{}/file/radio-paje-music/{}", auth.api_url, file_name);
        log::info!("Downloading file from {}", download_url);

        let bytes = reqwest::Client::new()
            .get(&download_url)
            .send()
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?
            .bytes()
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;

        log::info!("Downloaded {} bytes", bytes.len());

        fs::write(&file_path, &bytes)?;
        log::info!("Cached file to {}", file_path.display());
    } else {
        log::info!("Using cached file at {}", file_path.display());
    }

    // Detect MIME type from filename
    let mime_type = from_path(&file_name).first_or_octet_stream();

    // Log the request details
    let range_header = req.headers().get("range");
    log::info!("Range header: {:?}", range_header);
    log::info!("File size: {} bytes", file_path.metadata()?.len());

    // Use NamedFile which handles range requests automatically
    let file = NamedFile::open(&file_path)?
        .set_content_type(mime_type)
        .disable_content_disposition();

    Ok(file.into_response(&req))
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    log::info!("Starting server");

    let playback_state = Arc::new(Mutex::new(PlaybackState {
        is_playing: false,
        queue: VecDeque::new(),
    }));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(playback_state.clone()))
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
