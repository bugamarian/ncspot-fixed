use std::fs;
use std::io::{self, Write};

use librespot_core::authentication::Credentials as LibrespotCredentials;
use librespot_core::cache::Cache;
use librespot_oauth::OAuthClientBuilder;
use log::{debug, error, info};
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::{AuthCodeSpotify, Config as RspotifyConfig, Credentials, OAuth, Token};

use crate::client_config::ClientConfig;
use crate::config::{self, Config};
use crate::redirect_uri::redirect_uri_web_server;
use crate::spotify::Spotify;

pub const SPOTIFY_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";

const TOKEN_CACHE_FILE: &str = ".spotify_token_cache.json";

pub static OAUTH_SCOPES: &[&str] = &[
    "playlist-read-collaborative",
    "playlist-read-private",
    "playlist-modify-private",
    "playlist-modify-public",
    "user-follow-read",
    "user-follow-modify",
    "user-library-modify",
    "user-library-read",
    "user-modify-playback-state",
    "user-read-currently-playing",
    "user-read-playback-state",
    "user-read-playback-position",
    "user-read-private",
    "user-read-recently-played",
    "user-top-read",
    "streaming",
];

pub struct AuthResult {
    pub librespot_credentials: LibrespotCredentials,
    pub web_api: AuthCodeSpotify,
}

fn get_token_cache_path() -> std::path::PathBuf {
    config::config_path(TOKEN_CACHE_FILE)
}

fn save_token_to_file(spotify: &AuthCodeSpotify) -> Result<(), String> {
    let token_lock = spotify.token.lock().ok().ok_or("Failed to lock token")?;
    if let Some(ref token) = *token_lock {
        let token_json =
            serde_json::to_string_pretty(token).map_err(|e| format!("Serialize error: {}", e))?;
        let path = get_token_cache_path();
        fs::write(&path, token_json).map_err(|e| format!("Write error: {}", e))?;
        debug!("Token saved to {:?}", path);
    }
    Ok(())
}

fn load_token_from_file(spotify: &AuthCodeSpotify) -> Result<bool, String> {
    let path = get_token_cache_path();
    if !path.exists() {
        return Ok(false);
    }

    let token_json = fs::read_to_string(&path).map_err(|e| format!("Read error: {}", e))?;
    let token: Token =
        serde_json::from_str(&token_json).map_err(|e| format!("Parse error: {}", e))?;

    let mut token_lock = spotify.token.lock().ok().ok_or("Failed to lock token")?;
    *token_lock = Some(token);
    drop(token_lock);

    info!("Loaded cached authentication token");
    Ok(true)
}

fn create_rspotify_client(client_config: &ClientConfig) -> AuthCodeSpotify {
    let creds = Credentials::new(&client_config.client_id, &client_config.client_secret);

    let oauth = OAuth {
        redirect_uri: client_config.get_redirect_uri(),
        scopes: OAUTH_SCOPES.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    };

    let config = RspotifyConfig {
        token_refreshing: true,
        ..Default::default()
    };

    AuthCodeSpotify::with_config(creds, oauth, config)
}

fn perform_oauth_flow(spotify: &mut AuthCodeSpotify, port: u16) -> Result<(), String> {
    let auth_url = spotify
        .get_authorize_url(false)
        .map_err(|e| format!("Failed to get auth URL: {}", e))?;

    println!("\nOpening authorization URL in your browser...");
    println!("{}\n", auth_url);

    if let Err(e) = open::that(&auth_url) {
        println!("Failed to open browser automatically: {}", e);
        println!("Please manually open the URL above in your browser.");
    }

    println!(
        "Waiting for authorization callback on http://127.0.0.1:{}...\n",
        port
    );

    match redirect_uri_web_server(port) {
        Ok(callback_url) => {
            if let Some(code) = spotify.parse_response_code(&callback_url) {
                spotify
                    .request_token(&code)
                    .map_err(|e| format!("Token request failed: {}", e))?;

                save_token_to_file(spotify)?;
                println!("Successfully authenticated with Spotify!");
                Ok(())
            } else {
                Err("Failed to parse authorization code from callback URL".to_string())
            }
        }
        Err(e) => {
            println!("Web server failed: {}. Falling back to manual input.", e);
            manual_auth_flow(spotify)
        }
    }
}

fn manual_auth_flow(spotify: &mut AuthCodeSpotify) -> Result<(), String> {
    let auth_url = spotify
        .get_authorize_url(false)
        .map_err(|e| format!("Failed to get auth URL: {}", e))?;

    println!("Please open this URL in your browser:");
    println!("{}\n", auth_url);
    print!("Enter the URL you were redirected to: ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read input: {}", e))?;

    if let Some(code) = spotify.parse_response_code(&input) {
        spotify
            .request_token(&code)
            .map_err(|e| format!("Token request failed: {}", e))?;

        save_token_to_file(spotify)?;
        Ok(())
    } else {
        Err("Failed to parse authorization code from input URL".to_string())
    }
}

pub fn authenticate(
    client_config: &ClientConfig,
    app_config: &Config,
) -> Result<AuthResult, String> {
    let mut spotify = create_rspotify_client(client_config);

    let needs_auth = match load_token_from_file(&spotify) {
        Ok(true) => {
            let token_lock = spotify.token.lock().ok().ok_or("Failed to lock token")?;
            if let Some(ref token) = *token_lock {
                let is_expired = token.is_expired();
                drop(token_lock);
                if is_expired {
                    info!("Cached token is expired, need to refresh");
                    match spotify.refresh_token() {
                        Ok(()) => {
                            save_token_to_file(&spotify)?;
                            false
                        }
                        Err(e) => {
                            error!("Token refresh failed: {}, need to re-authenticate", e);
                            true
                        }
                    }
                } else {
                    false
                }
            } else {
                drop(token_lock);
                true
            }
        }
        Ok(false) => {
            info!("No cached token found, need to authenticate");
            true
        }
        Err(e) => {
            error!("Failed to read token cache: {}", e);
            true
        }
    };

    if needs_auth {
        perform_oauth_flow(&mut spotify, client_config.get_port())?;
    }

    let librespot_credentials = get_librespot_credentials(client_config, app_config)?;

    Ok(AuthResult {
        librespot_credentials,
        web_api: spotify,
    })
}

fn get_librespot_credentials(
    client_config: &ClientConfig,
    configuration: &Config,
) -> Result<LibrespotCredentials, String> {
    let cache = Cache::new(Some(config::cache_path("librespot")), None, None, None)
        .expect("Could not create librespot cache");

    if let Some(cached) = cache.credentials() {
        info!("Using cached librespot credentials");
        if Spotify::test_credentials(configuration, cached.clone()).is_ok() {
            return Ok(cached);
        }
        info!("Cached librespot credentials invalid, getting new ones");
    }

    info!("Getting librespot credentials via OAuth");
    create_librespot_credentials(client_config)
}

fn create_librespot_credentials(
    client_config: &ClientConfig,
) -> Result<LibrespotCredentials, String> {
    let redirect_uri = client_config.get_redirect_uri();

    let client_builder = OAuthClientBuilder::new(
        &client_config.client_id,
        &redirect_uri,
        OAUTH_SCOPES.to_vec(),
    );
    let oauth_client = client_builder.build().map_err(|e| e.to_string())?;

    oauth_client
        .get_access_token()
        .map(|token| LibrespotCredentials::with_access_token(token.access_token))
        .map_err(|e| e.to_string())
}
