//! Tiny, read-only, loopback-only HTTP service for the Chromium resource map.
//! This is an alpha transport. A local authenticated IPC service is the target architecture.

use crate::resources::Monitor;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread,
    time::Duration,
};

const INDEX_HTML: &str = include_str!("../../../apps/resource-map/index.html");
const APP_JS: &str = include_str!("../../../apps/resource-map/app.js");
const APP_CSS: &str = include_str!("../../../apps/resource-map/styles.css");

pub struct ResourceMap {
    pub url: String,
}

pub fn start(monitor: Arc<Monitor>) -> io::Result<ResourceMap> {
    // Port is selected by OS, and no listener is ever opened on a public interface.
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    let token = random_token()?;
    let url = format!("http://127.0.0.1:{port}/#token={token}");

    thread::spawn(move || {
        for incoming in listener.incoming() {
            match incoming {
                Ok(stream) => {
                    let token = token.clone();
                    let monitor = Arc::clone(&monitor);
                    thread::spawn(move || {
                        if let Err(error) = serve(stream, port, &token, &monitor) {
                            eprintln!("zh: resource map connection error: {error}");
                        }
                    });
                }
                Err(error) => eprintln!("zh: resource map listener error: {error}"),
            }
        }
    });

    Ok(ResourceMap { url })
}

fn random_token() -> io::Result<String> {
    let mut source = File::open("/dev/urandom")?;
    let mut bytes = [0u8; 32];
    source.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn serve(mut stream: TcpStream, port: u16, token: &str, monitor: &Monitor) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(4)))?;
    stream.set_write_timeout(Some(Duration::from_secs(4)))?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    if request_line.len() > 8192 {
        return response(&mut stream, "431 Request Header Fields Too Large", "text/plain", b"request too large");
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let mut host = String::new();
    let mut authorization = String::new();

    for _ in 0..80 {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" { break; }
        if line.len() > 8192 {
            return response(&mut stream, "431 Request Header Fields Too Large", "text/plain", b"request too large");
        }
        if let Some((key, value)) = line.split_once(':') {
            match key.to_ascii_lowercase().as_str() {
                "host" => host = value.trim().to_string(),
                "authorization" => authorization = value.trim().to_string(),
                _ => (),
            }
        }
    }

    let allowed_host = format!("127.0.0.1:{port}");
    if host != allowed_host {
        return response(&mut stream, "403 Forbidden", "text/plain", b"invalid host");
    }
    if method != "GET" {
        return response(&mut stream, "405 Method Not Allowed", "text/plain", b"read only");
    }

    match path {
        "/" | "/index.html" => response(&mut stream, "200 OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes()),
        "/app.js" => response(&mut stream, "200 OK", "application/javascript; charset=utf-8", APP_JS.as_bytes()),
        "/styles.css" => response(&mut stream, "200 OK", "text/css; charset=utf-8", APP_CSS.as_bytes()),
        "/api/v1/local" => {
            let expected = format!("Bearer {token}");
            if !constant_time_equals(authorization.as_bytes(), expected.as_bytes()) {
                return response(&mut stream, "401 Unauthorized", "application/json", b"{\"error\":\"unauthorized\"}");
            }
            let body = serde_json::to_vec(&monitor.snapshot()).unwrap_or_default();
            response(&mut stream, "200 OK", "application/json", &body)
        }
        _ => response(&mut stream, "404 Not Found", "text/plain", b"not found"),
    }
}

fn constant_time_equals(a: &[u8], b: &[u8]) -> bool {
    let mut difference = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        difference |= (a.get(i).copied().unwrap_or_default()
            ^ b.get(i).copied().unwrap_or_default()) as usize;
    }
    difference == 0
}

fn response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_have_256_bits_of_entropy() {
        let one = random_token().unwrap();
        let two = random_token().unwrap();
        assert_eq!(one.len(), 64);
        assert_ne!(one, two);
    }

    #[test]
    fn auth_comparison() {
        assert!(constant_time_equals(b"Bearer example", b"Bearer example"));
        assert!(!constant_time_equals(b"Bearer wrong", b"Bearer example"));
    }
}
