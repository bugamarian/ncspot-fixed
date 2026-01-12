use std::fs;
use std::io::{Write, stdin};
use std::path::PathBuf;

use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::config;

const CLIENT_CONFIG_FILE: &str = "client.yml";
const DEFAULT_PORT: u16 = 8888;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub device_id: Option<String>,
    pub port: Option<u16>,
}

pub struct ClientConfigPaths {
    pub config_file_path: PathBuf,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            device_id: None,
            port: None,
        }
    }

    pub fn get_redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.get_port())
    }

    pub fn get_port(&self) -> u16 {
        self.port.unwrap_or(DEFAULT_PORT)
    }

    pub fn get_or_build_paths(&self) -> Result<ClientConfigPaths, String> {
        let config_dir = config::user_configuration_directory()
            .ok_or_else(|| "Could not determine configuration directory".to_string())?;

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        Ok(ClientConfigPaths {
            config_file_path: config_dir.join(CLIENT_CONFIG_FILE),
        })
    }

    pub fn load_config(&mut self) -> Result<(), String> {
        let paths = self.get_or_build_paths()?;

        if paths.config_file_path.exists() {
            debug!("Loading client config from {:?}", paths.config_file_path);
            let config_string = fs::read_to_string(&paths.config_file_path)
                .map_err(|e| format!("Failed to read config: {}", e))?;
            let config_yml: Self = serde_yaml::from_str(&config_string)
                .map_err(|e| format!("Failed to parse config: {}", e))?;

            self.client_id = config_yml.client_id;
            self.client_secret = config_yml.client_secret;
            self.device_id = config_yml.device_id;
            self.port = config_yml.port;

            if self.client_id.is_empty() || self.client_secret.is_empty() {
                return Err("client_id or client_secret is empty in config file".to_string());
            }

            info!("Loaded client configuration");
            Ok(())
        } else {
            self.run_setup_wizard(&paths)
        }
    }

    fn run_setup_wizard(&mut self, paths: &ClientConfigPaths) -> Result<(), String> {
        println!("\n=== ncspot OAuth Setup ===\n");
        println!(
            "Config will be saved to: {}\n",
            paths.config_file_path.display()
        );

        println!("To use ncspot, you need to create a Spotify Developer application:\n");
        println!("  1. Go to https://developer.spotify.com/dashboard/applications");
        println!("  2. Click 'Create app' and fill in a name and description");
        println!(
            "  3. Add `http://127.0.0.1:{}/callback` to Redirect URIs",
            DEFAULT_PORT
        );
        println!("  4. Save your app and copy the Client ID and Client Secret\n");

        let client_id = Self::get_client_key_from_input("Client ID")?;
        let client_secret = Self::get_client_key_from_input("Client Secret")?;

        println!("\nEnter port for redirect URI (default {}): ", DEFAULT_PORT);
        let mut port_input = String::new();
        stdin()
            .read_line(&mut port_input)
            .map_err(|e| format!("Failed to read input: {}", e))?;
        let port = port_input.trim().parse::<u16>().unwrap_or(DEFAULT_PORT);

        let config_yml = Self {
            client_id: client_id.clone(),
            client_secret: client_secret.clone(),
            device_id: None,
            port: Some(port),
        };

        let content_yml = serde_yaml::to_string(&config_yml)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        let mut new_config = fs::File::create(&paths.config_file_path)
            .map_err(|e| format!("Failed to create config file: {}", e))?;
        write!(new_config, "{}", content_yml)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        self.client_id = client_id;
        self.client_secret = client_secret;
        self.device_id = None;
        self.port = Some(port);

        println!("\nConfiguration saved successfully!");
        println!(
            "Make sure your Redirect URI in Spotify Dashboard matches: http://127.0.0.1:{}/callback\n",
            port
        );

        Ok(())
    }

    fn get_client_key_from_input(type_label: &str) -> Result<String, String> {
        const MAX_RETRIES: u8 = 5;
        let mut num_retries = 0;

        loop {
            print!("Enter your {}: ", type_label);
            std::io::stdout().flush().ok();

            let mut client_key = String::new();
            stdin()
                .read_line(&mut client_key)
                .map_err(|e| format!("Failed to read input: {}", e))?;
            let client_key = client_key.trim().to_string();

            match Self::validate_client_key(&client_key) {
                Ok(()) => return Ok(client_key),
                Err(error_string) => {
                    println!("  Error: {}", error_string);
                    num_retries += 1;
                    if num_retries >= MAX_RETRIES {
                        return Err(format!("Maximum retries ({}) exceeded", MAX_RETRIES));
                    }
                }
            }
        }
    }

    fn validate_client_key(key: &str) -> Result<(), String> {
        const EXPECTED_LEN: usize = 32;

        if key.is_empty() {
            return Err("Key cannot be empty".to_string());
        }

        if key.len() != EXPECTED_LEN {
            return Err(format!(
                "Invalid length: {} (must be {})",
                key.len(),
                EXPECTED_LEN
            ));
        }

        if !key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid character found (must be hex digits only)".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_key() {
        let valid_key = "65b708073fc0480ea92a077233ca87bd";
        assert!(ClientConfig::validate_client_key(valid_key).is_ok());
    }

    #[test]
    fn test_validate_invalid_length() {
        let short_key = "65b708073fc0480e";
        assert!(ClientConfig::validate_client_key(short_key).is_err());
    }

    #[test]
    fn test_validate_invalid_chars() {
        let bad_key = "65b708073fc0480ea92a077233ca87zz";
        assert!(ClientConfig::validate_client_key(bad_key).is_err());
    }

    #[test]
    fn test_redirect_uri() {
        let config = ClientConfig {
            port: Some(8888),
            ..Default::default()
        };
        assert_eq!(config.get_redirect_uri(), "http://127.0.0.1:8888/callback");
    }
}
