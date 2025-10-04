// use aws_config::{BehaviorVersion, Region};
// use aws_sdk_s3::{Client, Config};
use rand::prelude::IndexedRandom;
use reqwest::Client;
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use std::error::Error;

use crate::auth::AuthResponse;

#[derive(Deserialize)]
struct FileListResponse {
    files: Vec<FileInfo>,
}

#[derive(Deserialize, Clone)]
struct FileInfo {
    #[serde(rename = "fileName")]
    file_name: String,
}

pub async fn select_random_file(
    auth: &AuthResponse,
    bucket_id: &str,
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let url = format!("{}/b2api/v4/b2_list_file_names", auth.api_url);

    log::info!("Fetching from url: {}", url);

    let params = [("bucketId", bucket_id), ("maxFileCount", "100")];

    let query_string = serde_urlencoded::to_string(&params)?;
    let url_with_params = format!("{url}?{query_string}");

    log::info!("Fetching from url: {}", url_with_params);

    let response = client
        .get(&url_with_params)
        .header(AUTHORIZATION, &auth.authorization_token)
        .send()
        .await?;

    log::debug!("Response: {:?}", response);

    let text = response.text().await?;
    log::debug!("Raw file list response: {}", text);

    let file_list: FileListResponse = serde_json::from_str(&text).map_err(|e| {
        log::error!("Failed to parse file list response: {}", e);
        actix_web::error::ErrorInternalServerError("File list parse error")
    })?;

    let files = file_list.files;
    let mut rng = rand::rng();
    let selected = files.choose(&mut rng).ok_or("No files found")?;

    Ok(selected.file_name.clone())
}

// pub async fn select_random_file(bucket_name: &str) -> Result<String, Box<dyn Error>> {
//     let region = Region::new("us-east-005");

//     let shared_config = aws_config::defaults(BehaviorVersion::v2025_08_07())
//         .region(region.clone())
//         .endpoint_url("https://s3.us-east-005.backblazeb2.com")
//         .load()
//         .await;

//     let s3_config = Config::builder()
//         .region(region)
//         .credentials_provider(shared_config.credentials_provider().unwrap())
//         .endpoint_url("https://s3.us-east-005.backblazeb2.com")
//         .force_path_style(true)
//         .build();

//     let client = Client::new(s3_config);

//     log::info!("Fetching from bucket {}", bucket_name);

//     let resp = client
//         .list_objects_v2()
//         .bucket(bucket_name)
//         .max_keys(100)
//         .send()
//         .await?;

//     let files = resp.contents();
//     if files.is_empty() {
//         return Err("No files found".into());
//     }

//     let mut rng = rand::rng();
//     let selected = files.choose(&mut rng).ok_or("No files found")?;

//     Ok(selected.key().unwrap_or_default().to_string())
// }
