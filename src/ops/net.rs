//! Network Operations (Ops)
//!
//! This module provides network operations that can be called from JavaScript.
//! Includes HTTP client functionality with permission checks.

use std::collections::HashMap;
use std::net::ToSocketAddrs;
use thiserror::Error;

use crate::permissions::{Permissions, PermissionError};

/// Errors that can occur during network operations
#[derive(Error, Debug)]
pub enum NetError {
    /// Permission denied for network operation
    #[error("Permission error: {0}")]
    Permission(#[from] PermissionError),

    /// Invalid URL format
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// HTTP request failed
    #[error("Request failed: {0}")]
    RequestFailed(String),

    /// Request timeout
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Invalid HTTP response
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Connection error occurred
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// DNS resolution failed
    #[error("DNS resolution failed: {0}")]
    DnsError(String),
}

/// Result type for network operations
pub type NetResult<T> = Result<T, NetError>;

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    /// GET request method
    GET,
    /// POST request method
    POST,
    /// PUT request method
    PUT,
    /// DELETE request method
    DELETE,
    /// PATCH request method
    PATCH,
    /// HEAD request method
    HEAD,
    /// OPTIONS request method
    OPTIONS,
}

impl HttpMethod {
    /// Parse HTTP method from string
    pub fn from_str(method: &str) -> Option<Self> {
        match method.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::GET),
            "POST" => Some(HttpMethod::POST),
            "PUT" => Some(HttpMethod::PUT),
            "DELETE" => Some(HttpMethod::DELETE),
            "PATCH" => Some(HttpMethod::PATCH),
            "HEAD" => Some(HttpMethod::HEAD),
            "OPTIONS" => Some(HttpMethod::OPTIONS),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
        }
    }
}

/// HTTP request headers
pub type HttpHeaders = HashMap<String, String>;

/// HTTP request configuration
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// Request method
    pub method: Option<HttpMethod>,
    /// Request headers
    pub headers: Option<HttpHeaders>,
    /// Request body
    pub body: Option<Vec<u8>>,
    /// Request timeout in milliseconds
    pub timeout: Option<u64>,
    /// Whether to follow redirects
    pub redirect: Option<bool>,
    /// Maximum redirect depth
    pub max_redirects: Option<usize>,
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct FetchResponse {
    /// Status code
    pub status: u16,
    /// Status text
    pub status_text: String,
    /// Response headers
    pub headers: HttpHeaders,
    /// Response body
    pub body: Vec<u8>,
    /// URL (after redirects)
    pub url: String,
}

impl FetchResponse {
    /// Get the response body as text
    pub fn text(&self) -> NetResult<String> {
        String::from_utf8(self.body.clone())
            .map_err(|_| NetError::InvalidResponse("Response is not valid UTF-8".into()))
    }

    /// Get the response body as JSON
    pub fn json(&self) -> NetResult<serde_json::Value> {
        serde_json::from_slice(&self.body)
            .map_err(|e| NetError::InvalidResponse(format!("Invalid JSON: {}", e)))
    }

    /// Check if the response was successful (2xx status code)
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Parse a URL to extract the hostname for permission checking
fn extract_hostname(url: &str) -> NetResult<String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        let parsed = url::Url::parse(url)
            .map_err(|_| NetError::InvalidUrl(url.to_string()))?;

        let hostname = parsed.host_str()
            .ok_or_else(|| NetError::InvalidUrl("No hostname in URL".into()))?;

        Ok(hostname.to_string())
    } else {
        Err(NetError::InvalidUrl("URL must start with http:// or https://".into()))
    }
}

/// Check permissions for a URL
fn check_url_permissions(url: &str, permissions: &Permissions) -> NetResult<()> {
    let hostname = extract_hostname(url)?;
    permissions.check_net(&hostname)?;
    Ok(())
}

/// Fetch a URL using HTTP
///
/// This is a synchronous implementation. In production, use async HTTP client.
pub fn fetch(url: &str, options: Option<FetchOptions>, permissions: &Permissions) -> NetResult<FetchResponse> {
    // Check permissions
    check_url_permissions(url, permissions)?;

    let _opts = options.unwrap_or_default();

    // Parse URL
    let _parsed_url = url::Url::parse(url)
        .map_err(|_| NetError::InvalidUrl(url.to_string()))?;

    // Build HTTP client (using ureq for simplicity - add to Cargo.toml)
    // For now, this is a placeholder implementation

    // TODO: Implement actual HTTP fetch using reqwest or hyper
    // This is a simplified version that demonstrates the API

    Err(NetError::RequestFailed("HTTP fetch not yet fully implemented. Please add reqwest or ureq to Cargo.toml".into()))
}

/// Fetch a URL and return the response as text
pub fn fetch_text(url: &str, options: Option<FetchOptions>, permissions: &Permissions) -> NetResult<String> {
    let response = fetch(url, options, permissions)?;
    response.text()
}

/// Fetch a URL and return the response as JSON
pub fn fetch_json(url: &str, options: Option<FetchOptions>, permissions: &Permissions) -> NetResult<serde_json::Value> {
    let response = fetch(url, options, permissions)?;
    response.json()
}

/// TCP connection information
#[derive(Debug, Clone)]
pub struct TcpConnection {
    /// Local address of the connection
    pub local_addr: String,
    /// Peer (remote) address of the connection
    pub peer_addr: String,
}

/// Connect to a TCP address
pub fn tcp_connect(address: &str, permissions: &Permissions) -> NetResult<TcpConnection> {
    // Parse address to get hostname
    let hostname = if let Some(host) = address.split(':').next() {
        host
    } else {
        address
    };

    // Check permissions
    permissions.check_net(hostname)?;

    // TODO: Implement actual TCP connection
    Err(NetError::ConnectionError("TCP connection not yet implemented".into()))
}

/// Resolve a hostname to IP addresses
pub fn dns_lookup(hostname: &str, permissions: &Permissions) -> NetResult<Vec<String>> {
    // Check permissions
    permissions.check_net(hostname)?;

    // Use standard library's DNS resolution
    let addresses: Vec<std::net::SocketAddr> = format!("{}:0", hostname)
        .to_socket_addrs()
        .map_err(|e| NetError::DnsError(format!("Failed to resolve {}: {}", hostname, e)))?
        .collect();

    let mut ips = Vec::new();
    for addr in addresses {
        ips.push(addr.ip().to_string());
    }

    // Deduplicate
    ips.sort();
    ips.dedup();

    Ok(ips)
}

/// WebSocket connection (placeholder for future implementation)
pub struct WebSocketConnection {
    _url: String,
}

impl WebSocketConnection {
    /// Connect to a WebSocket server
    pub fn connect(url: &str, permissions: &Permissions) -> NetResult<Self> {
        // Check permissions
        check_url_permissions(url, permissions)?;

        // TODO: Implement WebSocket connection
        Err(NetError::ConnectionError("WebSocket not yet implemented".into()))
    }

    /// Send a message
    pub fn send(&mut self, _message: &str) -> NetResult<()> {
        // TODO: Implement WebSocket send
        Err(NetError::ConnectionError("WebSocket not yet implemented".into()))
    }

    /// Receive a message
    pub fn recv(&mut self) -> NetResult<String> {
        // TODO: Implement WebSocket receive
        Err(NetError::ConnectionError("WebSocket not yet implemented".into()))
    }

    /// Close the connection
    pub fn close(self) -> NetResult<()> {
        // TODO: Implement WebSocket close
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn test_perms() -> Permissions {
        Permissions::allow_all()
    }

    #[allow(dead_code)]
    fn no_perms() -> Permissions {
        Permissions::default()
    }

    #[test]
    fn test_http_method_from_str() {
        assert_eq!(HttpMethod::from_str("GET"), Some(HttpMethod::GET));
        assert_eq!(HttpMethod::from_str("get"), Some(HttpMethod::GET));
        assert_eq!(HttpMethod::from_str("POST"), Some(HttpMethod::POST));
        assert_eq!(HttpMethod::from_str("INVALID"), None);
    }

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::GET.as_str(), "GET");
        assert_eq!(HttpMethod::POST.as_str(), "POST");
        assert_eq!(HttpMethod::DELETE.as_str(), "DELETE");
    }

    #[test]
    fn test_extract_hostname() {
        assert_eq!(extract_hostname("https://example.com/path").unwrap(), "example.com");
        assert_eq!(extract_hostname("http://api.example.com:8080/v1").unwrap(), "api.example.com");
        assert!(extract_hostname("ftp://example.com").is_err());
        assert!(extract_hostname("not-a-url").is_err());
    }

    #[test]
    fn test_check_url_permissions_allowed() {
        let perms = Permissions::allow_all();
        assert!(check_url_permissions("https://example.com", &perms).is_ok());
    }

    #[test]
    fn test_check_url_permissions_denied() {
        let perms = Permissions::default();
        assert!(matches!(
            check_url_permissions("https://example.com", &perms),
            Err(NetError::Permission(_))
        ));
    }

    #[test]
    fn test_dns_lookup_allowed() {
        let perms = Permissions::allow_all();
        // Use a well-known DNS address
        let result = dns_lookup("localhost", &perms);
        assert!(result.is_ok());
        let ips = result.unwrap();
        // Should resolve to 127.0.0.1 or ::1
        assert!(ips.contains(&"127.0.0.1".to_string()) || ips.contains(&"::1".to_string()));
    }

    #[test]
    fn test_dns_lookup_denied() {
        let perms = Permissions::default();
        let result = dns_lookup("example.com", &perms);
        assert!(matches!(result, Err(NetError::Permission(_))));
    }

    #[test]
    fn test_fetch_response_ok() {
        let response = FetchResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: HttpHeaders::new(),
            body: b"Hello, World!".to_vec(),
            url: "https://example.com".to_string(),
        };

        assert!(response.ok());
        assert_eq!(response.text().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_fetch_response_not_ok() {
        let response = FetchResponse {
            status: 404,
            status_text: "Not Found".to_string(),
            headers: HttpHeaders::new(),
            body: b"Not Found".to_vec(),
            url: "https://example.com".to_string(),
        };

        assert!(!response.ok());
    }

    #[test]
    fn test_fetch_response_json() {
        let response = FetchResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: HttpHeaders::new(),
            body: br#"{"hello": "world"}"#.to_vec(),
            url: "https://example.com".to_string(),
        };

        let json = response.json().unwrap();
        assert_eq!(json["hello"], "world");
    }

    #[test]
    fn test_fetch_response_invalid_json() {
        let response = FetchResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: HttpHeaders::new(),
            body: b"not json".to_vec(),
            url: "https://example.com".to_string(),
        };

        assert!(response.json().is_err());
    }

    #[test]
    fn test_fetch_options_default() {
        let opts = FetchOptions::default();
        assert!(opts.method.is_none());
        assert!(opts.headers.is_none());
        assert!(opts.body.is_none());
        assert!(opts.timeout.is_none());
        assert!(opts.redirect.is_none());
        assert!(opts.max_redirects.is_none());
    }
}
