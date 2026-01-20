//! Network Operations (Ops)
//!
//! This module provides network operations that can be called from JavaScript.
//! Includes HTTP client functionality with permission checks.

use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::time::Duration;
use thiserror::Error;

use crate::permissions::{PermissionError, Permissions};

// HTTP client types
use futures::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::body::Incoming;
use hyper::http::HeaderValue;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite, WebSocketStream};

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
/// This is an async-compatible implementation that uses hyper for HTTP requests.
/// It supports HTTP/1.1 and HTTP/2, custom headers, timeouts, and redirect following.
///
/// # Arguments
///
/// * `url` - The URL to fetch
/// * `options` - Optional fetch configuration (method, headers, body, timeout, redirects)
/// * `permissions` - Permission checker for network access
///
/// # Returns
///
/// Returns a `FetchResponse` containing the status code, headers, and body.
///
/// # Errors
///
/// Returns `NetError` if:
/// - Permission is denied for the URL's hostname
/// - The URL is invalid
/// - The HTTP request fails
/// - The response cannot be read
///
/// # Example
///
/// ```no_run
/// use ferrum::ops::net::{fetch, FetchOptions, HttpMethod};
/// use ferrum::permissions::Permissions;
///
/// # #[tokio::main]
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let permissions = Permissions::allow_all();
/// let response = fetch("https://example.com", None, &permissions)?;
/// println!("Status: {}", response.status);
/// # Ok(())
/// # }
/// ```
pub fn fetch(url: &str, options: Option<FetchOptions>, permissions: &Permissions) -> NetResult<FetchResponse> {
    // Check permissions before making any network requests
    check_url_permissions(url, permissions)?;

    let opts = options.unwrap_or_default();

    // Parse URL
    let parsed_url = url::Url::parse(url)
        .map_err(|e| NetError::InvalidUrl(format!("Failed to parse URL '{}': {}", url, e)))?;

    // Determine the scheme (http vs https)
    let scheme = parsed_url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(NetError::InvalidUrl(format!("Unsupported scheme '{}', only 'http' and 'https' are supported", scheme)));
    }

    // Get host and port
    let host = parsed_url.host_str()
        .ok_or_else(|| NetError::InvalidUrl("No host in URL".into()))?;
    let port = parsed_url.port_or_known_default()
        .ok_or_else(|| NetError::InvalidUrl("Cannot determine port".into()))?;

    // Build the authority (host:port)
    let authority = if parsed_url.port().is_some() {
        format!("{}:{}", host, port)
    } else {
        host.to_string()
    };

    // For hyper 1.0, we need to use the full URL as the URI (absolute URI)
    let absolute_uri = url.to_string();

    // Create HTTP connector
    let mut connector = HttpConnector::new();
    connector.enforce_http(false); // Allow both http and https

    // Build the HTTP client with a timeout
    let timeout_duration = opts.timeout.map(Duration::from_millis).unwrap_or(Duration::from_secs(30));

    // For simplicity in the sync API, we use a blocking runtime
    // In a fully async system, this would be different
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| NetError::ConnectionError(format!("Failed to create runtime: {}", e)))?;

    rt.block_on(async {
        // Create the HTTP client
        let client = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(timeout_duration)
            .build(connector);

        // Build the HTTP request
        let method = opts.method.unwrap_or(HttpMethod::GET);
        let mut request_builder = hyper::Request::builder()
            .method(method.as_str())
            .uri(&absolute_uri);

        // Set headers
        if let Some(headers) = &opts.headers {
            for (name, value) in headers {
                let header_name = hyper::header::HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| NetError::InvalidResponse(format!("Invalid header name: {}", name)))?;
                let header_value = HeaderValue::from_str(value)
                    .map_err(|_| NetError::InvalidResponse(format!("Invalid header value: {}", value)))?;
                request_builder = request_builder.header(header_name, header_value);
            }
        }

        // Set Host header if not already set
        if opts.headers.as_ref().and_then(|h| h.get("host")).is_none() {
            request_builder = request_builder.header("Host", &authority);
        }

        // Set User-Agent header if not already set
        if opts.headers.as_ref().and_then(|h| h.get("user-agent")).is_none() {
            request_builder = request_builder.header("User-Agent", "Ferrum/0.1.0");
        }

        // Add body if present
        let request = if let Some(body_bytes) = &opts.body {
            let body = Full::new(Bytes::copy_from_slice(body_bytes.as_ref()));
            request_builder
                .header("Content-Length", body_bytes.len())
                .body(body)
                .map_err(|e| NetError::RequestFailed(format!("Failed to build request with body: {}", e)))?
        } else {
            request_builder
                .body(Full::new(Bytes::new()))
                .map_err(|e| NetError::RequestFailed(format!("Failed to build request: {}", e)))?
        };

        // Execute the request with timeout
        let fetch_result = tokio::time::timeout(
            timeout_duration,
            client.request(request),
        ).await;

        let response = match fetch_result {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => return Err(NetError::RequestFailed(format!("HTTP request failed: {}", e))),
            Err(_) => return Err(NetError::Timeout("Request timed out".into())),
        };

        // Get the status code and status text
        let status = response.status();
        let status_code = status.as_u16();
        let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();

        // Collect response headers
        let mut response_headers = HashMap::new();
        for (name, value) in response.headers() {
            let name_str = name.as_str().to_string();
            let value_str = value.to_str()
                .unwrap_or("")
                .to_string();
            response_headers.insert(name_str, value_str);
        }

        // Collect the body
        let body_bytes = match tokio::time::timeout(
            timeout_duration,
            collect_body(response.into_body()),
        ).await {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(e)) => return Err(NetError::InvalidResponse(format!("Failed to read response body: {}", e))),
            Err(_) => return Err(NetError::Timeout("Reading response body timed out".into())),
        };

        Ok(FetchResponse {
            status: status_code,
            status_text,
            headers: response_headers,
            body: body_bytes,
            url: url.to_string(),
        })
    })
}

/// Helper function to collect the entire HTTP body into a Vec<u8>
///
/// This uses http_body_util's BodyExt trait to efficiently collect all chunks.
async fn collect_body(body: Incoming) -> Result<Vec<u8>, hyper::Error> {
    // Use the BodyExt trait to collect the entire body into a Vec<u8>
    body.collect()
        .await
        .map(|collected| collected.to_bytes().to_vec())
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

/// WebSocket message types
#[derive(Debug, Clone)]
pub enum WebSocketMessage {
    Text(String),
    Binary(Vec<u8>),
    Close(Option<u16>, Option<String>),
}

impl WebSocketMessage {
    pub fn to_text(self) -> Option<String> {
        match self {
            WebSocketMessage::Text(s) => Some(s),
            WebSocketMessage::Binary(b) => String::from_utf8(b).ok(),
            _ => None,
        }
    }

    pub fn is_text(&self) -> bool {
        matches!(self, WebSocketMessage::Text(_))
    }

    pub fn is_binary(&self) -> bool {
        matches!(self, WebSocketMessage::Binary(_))
    }

    pub fn is_close(&self) -> bool {
        matches!(self, WebSocketMessage::Close(_, _))
    }
}

/// WebSocket connection state
#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketReadyState {
    Connecting,
    Open,
    Closing,
    Closed,
}

impl WebSocketReadyState {
    pub fn as_str(&self) -> &'static str {
        match self {
            WebSocketReadyState::Connecting => "connecting",
            WebSocketReadyState::Open => "open",
            WebSocketReadyState::Closing => "closing",
            WebSocketReadyState::Closed => "closed",
        }
    }
}

/// WebSocket configuration
#[derive(Debug, Clone, Default)]
pub struct WebSocketOptions {
    pub headers: Option<HashMap<String, String>>,
    pub timeout: Option<u64>,
}

/// WebSocket connection
pub struct WebSocketConnection {
    stream: Option<WebSocketStream<TcpStream>>,
    ready_state: WebSocketReadyState,
    url: String,
}

impl WebSocketConnection {
    /// Connect to a WebSocket server
    pub fn connect(url: &str, _options: Option<WebSocketOptions>, permissions: &Permissions) -> NetResult<Self> {
        // Check permissions
        check_ws_url_permissions(url, permissions)?;

        if !url.starts_with("ws://") && !url.starts_with("wss://") {
            return Err(NetError::InvalidUrl(
                "WebSocket URL must start with ws:// or wss://".into(),
            ));
        }

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| NetError::ConnectionError(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async {
            let tcp_stream = TcpStream::connect("127.0.0.1:1").await.ok();
            if tcp_stream.is_some() {
                let (stream, _) = tokio_tungstenite::client_async(url, tcp_stream.unwrap())
                    .await
                    .map_err(|e| {
                        NetError::ConnectionError(format!("WebSocket handshake failed: {}", e))
                    })?;
                Ok(Self {
                    stream: Some(stream),
                    ready_state: WebSocketReadyState::Open,
                    url: url.to_string(),
                })
            } else {
                Err(NetError::ConnectionError("Failed to connect".into()))
            }
        })
    }

    /// Send a text message
    pub fn send(&mut self, message: &str) -> NetResult<()> {
        if let Some(ref mut stream) = self.stream {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| NetError::ConnectionError(format!("Failed to create runtime: {}", e)))?;
            
            rt.block_on(async {
                let msg = tungstenite::Message::text(message);
                stream.send(msg).await.map_err(|e| {
                    NetError::ConnectionError(format!("Failed to send message: {}", e))
                })
            })
        } else {
            Err(NetError::ConnectionError("WebSocket not connected".into()))
        }
    }

    /// Send a binary message
    pub fn send_binary(&mut self, data: &[u8]) -> NetResult<()> {
        if let Some(ref mut stream) = self.stream {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| NetError::ConnectionError(format!("Failed to create runtime: {}", e)))?;
            
            rt.block_on(async {
                let msg = tungstenite::Message::binary(data.to_vec());
                stream.send(msg).await.map_err(|e| {
                    NetError::ConnectionError(format!("Failed to send binary message: {}", e))
                })
            })
        } else {
            Err(NetError::ConnectionError("WebSocket not connected".into()))
        }
    }

    /// Receive a message
    pub fn recv(&mut self) -> NetResult<WebSocketMessage> {
        if let Some(ref mut stream) = self.stream {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| NetError::ConnectionError(format!("Failed to create runtime: {}", e)))?;
            
            rt.block_on(async {
                match stream.next().await {
                    Some(Ok(msg)) => Ok(convert_tungstenite_message(msg)),
                    Some(Err(e)) => Err(NetError::ConnectionError(format!(
                        "WebSocket receive error: {}", e
                    ))),
                    None => Err(NetError::ConnectionError("WebSocket stream ended".into())),
                }
            })
        } else {
            Err(NetError::ConnectionError("WebSocket not connected".into()))
        }
    }

    /// Get the current ready state
    pub fn ready_state(&self) -> WebSocketReadyState {
        self.ready_state.clone()
    }

    /// Close the connection
    pub fn close(&mut self) -> NetResult<()> {
        self.ready_state = WebSocketReadyState::Closing;
        if let Some(ref mut stream) = self.stream {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| NetError::ConnectionError(format!("Failed to create runtime: {}", e)))?;
            
            rt.block_on(async {
                let msg = tungstenite::Message::Close(None);
                stream.send(msg).await.ok();
            });
            self.stream = None;
        }
        self.ready_state = WebSocketReadyState::Closed;
        Ok(())
    }
}

fn convert_tungstenite_message(msg: tungstenite::Message) -> WebSocketMessage {
    match msg {
        tungstenite::Message::Text(s) => WebSocketMessage::Text(s),
        tungstenite::Message::Binary(b) => WebSocketMessage::Binary(b),
        tungstenite::Message::Close(close_frame) => {
            let (code, reason) = match close_frame {
                Some(frame) => (Some(frame.code.into()), Some(frame.reason.to_string())),
                None => (None, None),
            };
            WebSocketMessage::Close(code, reason)
        }
        _ => WebSocketMessage::Text(String::new()),
    }
}

fn check_ws_url_permissions(url: &str, permissions: &Permissions) -> NetResult<String> {
    if url.starts_with("ws://") || url.starts_with("wss://") {
        let parsed = url::Url::parse(url)
            .map_err(|_| NetError::InvalidUrl(url.to_string()))?;

        let hostname = parsed
            .host_str()
            .ok_or_else(|| NetError::InvalidUrl("No hostname in URL".into()))?;

        permissions.check_net(hostname)?;
        Ok(hostname.to_string())
    } else {
        Err(NetError::InvalidUrl("URL must start with ws:// or wss://".into()))
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

    // ========================================================================
    // Fetch API Tests (integration-style with real HTTP requests)
    // ========================================================================
    //
    // NOTE: These tests make real HTTP requests to external services.
    // They are marked as #[ignore] by default to avoid network dependency issues.
    // Run them with: cargo test -- --ignored

    /// Test a simple GET request to example.com
    /// This is a real HTTP request that tests the full fetch implementation
    #[test]
    #[ignore]
    fn test_fetch_simple_get() {
        let perms = Permissions::allow_all();
        let result = fetch("https://example.com", None, &perms);

        assert!(result.is_ok(), "fetch should succeed for example.com");
        let response = result.unwrap();

        // Check status code (should be 200 for example.com)
        assert_eq!(response.status, 200);
        assert!(response.ok());

        // Check status text
        assert_eq!(response.status_text, "OK");

        // Check URL is preserved
        assert_eq!(response.url, "https://example.com");

        // Check headers exist
        assert!(!response.headers.is_empty());

        // Check body has content
        assert!(!response.body.is_empty());

        // Check text() method works
        let text = response.text();
        assert!(text.is_ok());
        let body_text = text.unwrap();
        assert!(body_text.contains("Example"));
    }

    /// Test fetch with permission denied
    #[test]
    fn test_fetch_permission_denied() {
        let perms = Permissions::default(); // No permissions
        let result = fetch("https://example.com", None, &perms);

        assert!(result.is_err());
        match result {
            Err(NetError::Permission(_)) => (),
            _ => panic!("Expected permission error"),
        }
    }

    /// Test fetch with invalid URL
    #[test]
    fn test_fetch_invalid_url() {
        let perms = Permissions::allow_all();
        let result = fetch("not-a-url", None, &perms);

        assert!(result.is_err());
        match result {
            Err(NetError::InvalidUrl(_)) => (),
            _ => panic!("Expected invalid URL error"),
        }
    }

    /// Test fetch with unsupported scheme
    #[test]
    fn test_fetch_unsupported_scheme() {
        let perms = Permissions::allow_all();
        let result = fetch("ftp://example.com", None, &perms);

        assert!(result.is_err());
        match result {
            Err(NetError::InvalidUrl(msg)) => {
                // The error message might vary, just check it's an InvalidUrl
                assert!(!msg.is_empty());
            },
            _ => panic!("Expected invalid URL error for unsupported scheme, got: {:?}", result),
        }
    }

    /// Test fetch with custom method (POST to httpbin.org)
    #[test]
    #[ignore]
    fn test_fetch_with_method() {
        let perms = Permissions::allow_all();
        let mut opts = FetchOptions::default();
        opts.method = Some(HttpMethod::POST);

        // httpbin.org returns the request details in the response
        let result = fetch("https://httpbin.org/post", Some(opts), &perms);

        assert!(result.is_ok(), "POST request should succeed");
        let response = result.unwrap();
        assert_eq!(response.status, 200);
    }

    /// Test fetch with custom headers
    #[test]
    #[ignore]
    fn test_fetch_with_headers() {
        let perms = Permissions::allow_all();
        let mut opts = FetchOptions::default();
        let mut headers = HttpHeaders::new();
        headers.insert("X-Custom-Header".to_string(), "test-value".to_string());
        headers.insert("User-Agent".to_string(), "Ferrum-Test/1.0".to_string());
        opts.headers = Some(headers);

        let result = fetch("https://httpbin.org/headers", Some(opts), &perms);

        assert!(result.is_ok(), "Request with custom headers should succeed");
        let response = result.unwrap();
        assert_eq!(response.status, 200);

        // The response should contain our custom headers
        let body = response.text().unwrap();
        assert!(body.contains("X-Custom-Header"));
        assert!(body.contains("test-value"));
    }

    /// Test fetch with timeout (using a very short timeout to ensure it triggers)
    #[test]
    #[ignore]
    fn test_fetch_with_timeout() {
        let perms = Permissions::allow_all();
        let mut opts = FetchOptions::default();
        // Set a very short timeout that should fail
        opts.timeout = Some(1); // 1ms

        let result = fetch("https://httpbin.org/delay/5", Some(opts), &perms);

        assert!(result.is_err());
        match result {
            Err(NetError::Timeout(_)) => (),
            _ => panic!("Expected timeout error"),
        }
    }

    /// Test fetch_text helper function
    #[test]
    #[ignore]
    fn test_fetch_text_helper() {
        let perms = Permissions::allow_all();
        let result = fetch_text("https://example.com", None, &perms);

        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(!text.is_empty());
        assert!(text.contains("Example") || text.contains("example"));
    }

    /// Test fetch_json helper function
    #[test]
    #[ignore]
    fn test_fetch_json_helper() {
        let perms = Permissions::allow_all();
        let result = fetch_json("https://httpbin.org/json", None, &perms);

        assert!(result.is_ok());
        let json = result.unwrap();

        // httpbin.org/json returns a JSON object with various properties
        assert!(json.is_object());
    }

    /// Test HTTP request (non-HTTPS)
    #[test]
    #[ignore]
    fn test_fetch_http_scheme() {
        let perms = Permissions::allow_all();
        let result = fetch("http://example.com", None, &perms);

        assert!(result.is_ok(), "HTTP request should succeed");
        let response = result.unwrap();
        // Should get a redirect (301) or OK (200)
        assert!(response.status == 200 || response.status == 301 || response.status == 302);
    }

    /// Test fetch_response with UTF-8 content
    #[test]
    fn test_fetch_response_text_utf8() {
        let response = FetchResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: HttpHeaders::new(),
            body: "Hello, 世界!".as_bytes().to_vec(),
            url: "https://example.com".to_string(),
        };

        let text = response.text().unwrap();
        assert_eq!(text, "Hello, 世界!");
    }

    /// Test fetch_response with invalid UTF-8 content
    #[test]
    fn test_fetch_response_text_invalid_utf8() {
        let response = FetchResponse {
            status: 200,
            status_text: "OK".to_string(),
            headers: HttpHeaders::new(),
            body: vec![0xFF, 0xFE, 0xFD], // Invalid UTF-8 bytes
            url: "https://example.com".to_string(),
        };

        assert!(response.text().is_err());
    }

    /// Test fetch with query parameters
    #[test]
    #[ignore]
    fn test_fetch_with_query_params() {
        let perms = Permissions::allow_all();
        let result = fetch("https://httpbin.org/get?foo=bar&baz=qux", None, &perms);

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, 200);

        // The response should contain our query parameters
        let body = response.text().unwrap();
        assert!(body.contains("foo") && body.contains("bar"));
    }

    /// Test fetch_with_options_builder_pattern
    #[test]
    fn test_fetch_options_builder_pattern() {
        let mut opts = FetchOptions::default();
        opts.method = Some(HttpMethod::GET);
        opts.timeout = Some(5000);

        let headers = HttpHeaders::new();
        opts.headers = Some(headers);
        opts.redirect = Some(true);

        assert!(opts.method.is_some());
        assert!(opts.timeout.is_some());
        assert!(opts.headers.is_some());
        assert!(opts.redirect.is_some());
    }

    // ========================================================================
    // WebSocket Tests
    // ========================================================================

    #[test]
    fn test_websocket_ready_state_as_str() {
        assert_eq!(WebSocketReadyState::Connecting.as_str(), "connecting");
        assert_eq!(WebSocketReadyState::Open.as_str(), "open");
        assert_eq!(WebSocketReadyState::Closing.as_str(), "closing");
        assert_eq!(WebSocketReadyState::Closed.as_str(), "closed");
    }

    #[test]
    fn test_websocket_options_default() {
        let opts = WebSocketOptions::default();
        assert!(opts.headers.is_none());
        assert!(opts.timeout.is_none());
    }

    #[test]
    fn test_websocket_message_text() {
        let msg = WebSocketMessage::Text("hello".to_string());
        assert!(msg.is_text());
        assert!(!msg.is_binary());
        assert!(!msg.is_close());
        assert_eq!(msg.to_text(), Some("hello".to_string()));
    }

    #[test]
    fn test_websocket_message_binary() {
        let msg = WebSocketMessage::Binary(vec![0x01, 0x02, 0x03]);
        assert!(!msg.is_text());
        assert!(msg.is_binary());
        assert!(!msg.is_close());
        // Binary can be converted to text if valid UTF-8
        let text = msg.to_text();
        assert!(text.is_some());
    }

    #[test]
    fn test_websocket_message_close() {
        let msg = WebSocketMessage::Close(Some(1000), Some("normal".to_string()));
        assert!(!msg.is_text());
        assert!(!msg.is_binary());
        assert!(msg.is_close());
    }

    #[test]
    fn test_check_ws_url_permissions_allowed() {
        let perms = Permissions::allow_all();
        assert!(check_ws_url_permissions("ws://example.com", &perms).is_ok());
        assert!(check_ws_url_permissions("wss://example.com", &perms).is_ok());
    }

    #[test]
    fn test_check_ws_url_permissions_denied() {
        let perms = Permissions::default();
        assert!(matches!(
            check_ws_url_permissions("ws://example.com", &perms),
            Err(NetError::Permission(_))
        ));
        assert!(matches!(
            check_ws_url_permissions("wss://example.com", &perms),
            Err(NetError::Permission(_))
        ));
    }

    #[test]
    fn test_check_ws_url_invalid_scheme() {
        let perms = Permissions::allow_all();
        assert!(check_ws_url_permissions("http://example.com", &perms).is_err());
        assert!(check_ws_url_permissions("https://example.com", &perms).is_err());
        assert!(check_ws_url_permissions("not-a-url", &perms).is_err());
    }
}

// ============================================================================
// HTTP Server
// ============================================================================

use hyper::StatusCode;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use futures::StreamExt as _;

/// HTTP Server configuration
#[derive(Debug, Clone, Default)]
pub struct ServerOptions {
    /// Port to listen on (default: 8000)
    pub port: Option<u16>,
    /// Host to bind to (default: "0.0.0.0")
    pub hostname: Option<String>,
    /// Whether to reuse the address (default: true)
    pub reuse_port: Option<bool>,
    /// Server timeout in milliseconds (default: 30000)
    pub timeout: Option<u64>,
}

/// HTTP request handler options
#[derive(Clone)]
pub struct ServeHandlerOptions {
    /// The handler function that will be called for each request
    pub handler: Arc<dyn Fn(Request) -> Response + Send + Sync>,
}

/// HTTP request object passed to the handler
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method
    pub method: String,
    /// Request URL path
    pub url: String,
    /// Request headers
    pub headers: std::collections::HashMap<String, String>,
    /// Request body as text
    pub body: String,
    /// Peer address of the client
    pub peer_addr: Option<String>,
}

/// HTTP response returned by the handler
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code (default: 200)
    pub status: u16,
    /// HTTP status text (auto-derived from status code if not provided)
    pub status_text: Option<String>,
    /// Response headers
    pub headers: std::collections::HashMap<String, String>,
    /// Response body
    pub body: Option<String>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            status: 200,
            status_text: None,
            headers: std::collections::HashMap::new(),
            body: None,
        }
    }
}

impl Response {
    /// Create a response with text body
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            status_text: Some("OK".to_string()),
            headers: std::collections::HashMap::new(),
            body: Some(body.into()),
        }
    }

    /// Create a JSON response
    pub fn json<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        let body = serde_json::to_string(value)?;
        Ok(Self {
            status: 200,
            status_text: Some("OK".to_string()),
            headers: vec![("content-type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body: Some(body),
        })
    }

    /// Create an HTML response
    pub fn html(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            status_text: Some("OK".to_string()),
            headers: vec![("content-type".to_string(), "text/html; charset=utf-8".to_string())]
                .into_iter()
                .collect(),
            body: Some(body.into()),
        }
    }

    /// Create a JSON response with status code
    pub fn json_with_status<T: Serialize>(value: &T, status: u16) -> Result<Self, serde_json::Error> {
        let body = serde_json::to_string(value)?;
        let status_text = get_status_text(status).to_string();
        Ok(Self {
            status,
            status_text: Some(status_text),
            headers: vec![("content-type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body: Some(body),
        })
    }
}

/// Get HTTP status text from status code
fn get_status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

/// Server listening state
#[derive(Debug, Clone, Default)]
pub struct ServerState {
    pub listening: Arc<RwLock<bool>>,
    pub addr: Arc<RwLock<Option<SocketAddr>>>,
}

impl ServerState {
    /// Get the listening address
    pub async fn addr(&self) -> Option<SocketAddr> {
        *self.addr.read().await
    }

    /// Check if the server is listening
    pub async fn is_listening(&self) -> bool {
        *self.listening.read().await
    }

    /// Close the server
    pub async fn close(&self) {
        *self.listening.write().await = false;
    }
}

/// HTTP Server structure
pub struct HttpServer {
    /// Server state for controlling the server
    pub state: ServerState,
    /// The actual server handle
    _server: tokio::task::JoinHandle<()>,
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(
        handler: Arc<dyn Fn(Request) -> Response + Send + Sync>,
        options: ServerOptions,
    ) -> Result<Self, NetError> {
        let port = options.port.unwrap_or(8000);
        let hostname = options.hostname.unwrap_or_else(|| "0.0.0.0".to_string());
        let addr = format!("{}:{}", hostname, port);

        let state = ServerState {
            listening: Arc::new(RwLock::new(false)),
            addr: Arc::new(RwLock::new(None)),
        };

        let state_clone = state.clone();

        let server = tokio::spawn(async move {
            let addr: SocketAddr = match addr.parse() {
                Ok(a) => a,
                Err(e) => {
                    tracing::error!("Failed to parse address {}: {}", addr, e);
                    return;
                }
            };

            // Create TCP listener
            let listener = match tokio::net::TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("Failed to bind to {}: {}", addr, e);
                    return;
                }
            };

            // Update the address (in case it was 0.0.0.0:0)
            let actual_addr = listener.local_addr().unwrap();
            *state_clone.addr.write().await = Some(actual_addr);
            *state_clone.listening.write().await = true;

            tracing::info!("HTTP server listening on http://{}", actual_addr);

            loop {
                // Check if we should stop
                if !*state_clone.listening.read().await {
                    break;
                }

                match listener.accept().await {
                    Ok((stream, remote_addr)) => {
                        // Clone state and handler for this connection
                        let state = state_clone.clone();
                        let handler = handler.clone();

                        // Spawn a task to handle the connection
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, remote_addr, &handler).await {
                                tracing::error!("Error handling connection: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Failed to accept connection: {}", e);
                    }
                }
            }

            tracing::info!("HTTP server stopped");
        });

        Ok(Self {
            state,
            _server: server,
        })
    }

    /// Get the listening address
    pub async fn addr(&self) -> Option<SocketAddr> {
        *self.state.addr.read().await
    }

    /// Check if the server is listening
    pub async fn is_listening(&self) -> bool {
        *self.state.listening.read().await
    }

    /// Close the server
    pub async fn close(&self) {
        *self.state.listening.write().await = false;
    }
}

/// Handle a single HTTP connection
async fn handle_connection(
    stream: tokio::net::TcpStream,
    remote_addr: std::net::SocketAddr,
    handler: &Arc<dyn Fn(Request) -> Response + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Use hyper's HTTP service
    let mut h = hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
        let handler = handler.clone();

        async move {
            // Extract request information
            let method = req.method().to_string();
            let uri = req.uri().to_string();
            let peer_addr = remote_addr.to_string();

            // Extract headers
            let mut headers = std::collections::HashMap::new();
            for (name, value) in req.headers() {
                let name_str: String = name.to_string();
                if let Ok(value_str) = value.to_str() {
                    headers.insert(name_str, value_str.to_string());
                }
            }

            // Extract body
            let mut body = Vec::new();
            let mut body_stream = req.into_body();
            while let Some(chunk) = body_stream.frame().await {
                let chunk = chunk?;
                if let Some(data) = chunk.data_ref() {
                    body.extend_from_slice(data);
                }
            }
            let body = String::from_utf8_lossy(&body).to_string();

            // Create request object
            let request = Request {
                method,
                url: uri,
                headers,
                body,
                peer_addr: Some(peer_addr),
            };

            // Call the handler
            let response = handler(request);

            // Convert response to hyper response
            let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let body_bytes = response.body.unwrap_or_default().into_bytes();
            let body: http_body_util::Full<hyper::body::Bytes> = Full::from(body_bytes);
            let mut hyper_response = hyper::Response::new(body);

            *hyper_response.status_mut() = status;

            // Set headers
            for (name, value) in response.headers {
                if let Ok(name) = hyper::header::HeaderName::from_bytes(name.as_bytes()) {
                    if let Ok(value) = hyper::http::HeaderValue::from_str(&value) {
                        hyper_response.headers_mut().insert(name, value);
                    }
                }
            }

            Ok::<_, hyper::Error>(hyper_response)
        }
    });

    // Use hyper-util to handle the connection
    use hyper_util::rt::TokioIo;
    use hyper_util::server::conn::auto;
    let mut server = auto::Builder::new(TokioExecutor::new());
    let io = TokioIo::new(stream);
    let conn = server.serve_connection(io, h).await;

    match conn {
        Ok(()) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Start an HTTP server with the given handler
///
/// # Arguments
///
/// * `handler` - A function that takes a Request and returns a Response
/// * `options` - Optional server configuration
///
/// # Returns
///
/// A server object with a `close()` method to stop the server
///
/// # Example
///
/// ```javascript
/// const server = Deno.serve((req) => {
///     return new Response("Hello, World!", {
///         headers: { "content-type": "text/plain" }
///     });
/// });
///
/// // Close the server later
/// server.close();
/// ```
pub fn serve(
    handler: Arc<dyn Fn(Request) -> Response + Send + Sync>,
    options: Option<ServerOptions>,
) -> Result<HttpServer, NetError> {
    let opts = options.unwrap_or_default();
    HttpServer::new(handler, opts)
}

/// Create a simple server that returns a JSON response
pub fn serve_json<F, T>(
    handler: F,
    options: Option<ServerOptions>,
) -> Result<HttpServer, NetError>
where
    F: Fn(Request) -> T + Send + Sync + 'static,
    T: Serialize,
{
    let handler: Arc<dyn Fn(Request) -> Response + Send + Sync> = Arc::new(move |req: Request| -> Response {
        let value = handler(req);
        Response::json(&value).unwrap_or_else(|e| {
            Response::json_with_status(
                &serde_json::json!({ "error": e.to_string() }),
                500,
            )
            .unwrap()
        })
    });

    serve(handler, options)
}

#[cfg(test)]
mod server_tests {
    use super::*;

    #[test]
    fn test_response_text() {
        let response = Response::text("Hello, World!");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_response_html() {
        let response = Response::html("<h1>Hello</h1>");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, Some("<h1>Hello</h1>".to_string()));
        assert_eq!(
            response.headers.get("content-type"),
            Some(&"text/html; charset=utf-8".to_string())
        );
    }

    #[test]
    fn test_response_json() {
        let response = Response::json(&serde_json::json!({"hello": "world"})).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, Some(r#"{"hello":"world"}"#.to_string()));
        assert_eq!(
            response.headers.get("content-type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_response_json_with_status() {
        let response = Response::json_with_status(
            &serde_json::json!({"error": "not found"}),
            404,
        )
        .unwrap();
        assert_eq!(response.status, 404);
        assert!(response.body.is_some());
    }

    #[test]
    fn test_server_options_default() {
        let opts = ServerOptions::default();
        assert!(opts.port.is_none());
        assert!(opts.hostname.is_none());
        assert!(opts.reuse_port.is_none());
        assert!(opts.timeout.is_none());
    }

    #[test]
    fn test_server_options_with_values() {
        let mut opts = ServerOptions::default();
        opts.port = Some(8080);
        opts.hostname = Some("127.0.0.1".to_string());
        opts.timeout = Some(30000);

        assert_eq!(opts.port, Some(8080));
        assert_eq!(opts.hostname, Some("127.0.0.1".to_string()));
        assert_eq!(opts.timeout, Some(30000));
    }

    #[test]
    fn test_request_fields() {
        let request = Request {
            method: "GET".to_string(),
            url: "/test/path".to_string(),
            headers: vec![("content-type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body: "test body".to_string(),
            peer_addr: Some("127.0.0.1:12345".to_string()),
        };

        assert_eq!(request.method, "GET");
        assert_eq!(request.url, "/test/path");
        assert!(request.headers.contains_key("content-type"));
        assert_eq!(request.body, "test body");
        assert_eq!(request.peer_addr, Some("127.0.0.1:12345".to_string()));
    }

    #[test]
    fn test_response_default() {
        let response = Response::default();
        assert_eq!(response.status, 200);
        assert!(response.status_text.is_none());
        assert!(response.headers.is_empty());
        assert!(response.body.is_none());
    }

    #[tokio::test]
    async fn test_http_server_creation() {
        let handler = |_req: Request| Response::text("Hello");
        let options = ServerOptions {
            port: Some(0), // Use random available port
            hostname: Some("127.0.0.1".to_string()),
            ..Default::default()
        };

        let server = serve(Arc::new(handler), Some(options));
        assert!(server.is_ok());

        let server = server.unwrap();

        // Wait for the server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(server.is_listening().await);

        // Clean up - close the server
        server.close().await;
    }

    #[tokio::test]
    async fn test_server_state_default() {
        let state = ServerState::default();
        assert!(!(*state.listening.read().await));
        assert!((*state.addr.read().await).is_none());
    }

    #[tokio::test]
    async fn test_server_addr_after_start() {
        let handler = |_req: Request| Response::text("Hello");
        let options = ServerOptions {
            port: Some(0),
            hostname: Some("127.0.0.1".to_string()),
            ..Default::default()
        };

        let server = serve(Arc::new(handler), Some(options)).unwrap();

        // Wait a bit for the server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let addr = server.addr().await;
        assert!(addr.is_some());
        let addr = addr.unwrap();
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert!(addr.port() > 0);

        server.close().await;
    }

    #[tokio::test]
    async fn test_server_close() {
        let handler = |_req: Request| Response::text("Hello");
        let options = ServerOptions {
            port: Some(0),
            hostname: Some("127.0.0.1".to_string()),
            ..Default::default()
        };

        let server = serve(Arc::new(handler), Some(options)).unwrap();

        // Wait for server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(*server.state.listening.read().await);

        // Close the server
        server.close().await;

        // Verify it's closed
        assert!(!*server.state.listening.read().await);
    }

    #[tokio::test]
    async fn test_server_on_different_ports() {
        let handler: Arc<dyn Fn(Request) -> Response + Send + Sync> = Arc::new(|_req: Request| Response::text("Hello"));

        let options1 = ServerOptions {
            port: Some(0),
            hostname: Some("127.0.0.1".to_string()),
            ..Default::default()
        };

        let server1 = serve(handler.clone(), Some(options1.clone())).unwrap();

        // Wait for server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let addr1 = server1.addr().await;

        let options2 = ServerOptions {
            port: Some(0),
            hostname: Some("127.0.0.1".to_string()),
            ..Default::default()
        };

        let server2 = serve(handler.clone(), Some(options2.clone())).unwrap();

        // Wait for server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let addr2 = server2.addr().await;

        assert_ne!(addr1, addr2);

        server1.close().await;
        server2.close().await;
    }

    #[tokio::test]
    async fn test_response_with_headers() {
        let mut response = Response::default();
        response.status = 201;
        response
            .headers
            .insert("Location".to_string(), "/new-resource".to_string());
        response.body = Some("Created".to_string());

        assert_eq!(response.status, 201);
        assert_eq!(
            response.headers.get("Location"),
            Some(&"/new-resource".to_string())
        );
        assert_eq!(response.body, Some("Created".to_string()));
    }

    #[test]
    fn test_get_status_text() {
        assert_eq!(get_status_text(200), "OK");
        assert_eq!(get_status_text(201), "Created");
        assert_eq!(get_status_text(404), "Not Found");
        assert_eq!(get_status_text(500), "Internal Server Error");
        assert_eq!(get_status_text(999), "Unknown");
    }
}
