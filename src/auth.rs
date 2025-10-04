use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::error::Error;

#[derive(Deserialize, Debug)]
pub struct AuthResponse {
    #[serde(rename = "apiUrl")]
    pub api_url: String,

    #[serde(rename = "authorizationToken")]
    pub authorization_token: String,
}

pub async fn authenticate() -> Result<AuthResponse, Box<dyn Error>> {
    let key_id = env::var("B2_KEY_ID")?;
    let app_key = env::var("B2_APP_KEY")?;

    let client = Client::new();
    let response = client
        .get("https://api.backblazeb2.com/b2api/v2/b2_authorize_account")
        .basic_auth(key_id, Some(app_key))
        .send()
        .await?;

    let text = response.text().await?;
    log::info!("Raw auth response: {}", text);

    let auth_response: AuthResponse = serde_json::from_str(&text).map_err(|e| {
        log::error!("Failed to parse auth response: {}", e);
        actix_web::error::ErrorInternalServerError("Auth parse error")
    })?;

    // let stripped_token = rem_last(&auth_response.authorization_token);

    // let formatted_response: AuthResponse = AuthResponse {
    //     api_url: auth_response.api_url,
    //     authorization_token: stripped_token.to_string(),
    // };

    log::info!("Auth response: {:?}", auth_response);

    Ok(auth_response)
}

fn rem_last(value: &str) -> &str {
    let mut chars = value.chars();
    chars.next_back();
    chars.as_str()
}
