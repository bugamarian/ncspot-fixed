use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

pub fn redirect_uri_web_server(port: u16) -> Result<String, String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Some(url) = handle_connection(stream) {
                    return Ok(url);
                }
            }
            Err(e) => {
                return Err(format!("Connection error: {}", e));
            }
        }
    }

    Err("No valid callback received".to_string())
}

fn handle_connection(mut stream: TcpStream) -> Option<String> {
    let mut buffer = [0; 2048];
    let _ = stream.read(&mut buffer).ok()?;

    let request = String::from_utf8_lossy(&buffer);
    let split: Vec<&str> = request.split_whitespace().collect();

    if split.len() > 1 {
        let path = split[1];

        let host = request
            .lines()
            .find(|line| line.to_lowercase().starts_with("host:"))
            .and_then(|line| line.split(':').nth(1))
            .map(|h| h.trim())
            .unwrap_or("127.0.0.1:8888");

        let full_url = format!("http://{}{}", host, path);

        respond_with_success(stream);
        return Some(full_url);
    }

    respond_with_error("Malformed request".to_string(), stream);
    None
}

fn respond_with_success(mut stream: TcpStream) {
    let contents = include_str!("redirect_uri.html");

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        contents.len(),
        contents
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    std::thread::sleep(std::time::Duration::from_millis(100));
}

fn respond_with_error(error_message: String, mut stream: TcpStream) {
    let body = format!("400 - Bad Request - {}", error_message);
    let response = format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    std::thread::sleep(std::time::Duration::from_millis(100));
}
