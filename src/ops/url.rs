//! URL and URLSearchParams API
//!
//! This module provides URL and URLSearchParams implementations for the Ferrum runtime.
//! These are standard Web APIs for URL parsing and manipulation.

use std::collections::HashMap;
use std::default::Default;

/// URLSearchParams implementation
///
/// Provides methods for working with URL query strings.
///
/// # JavaScript API
/// ```javascript
/// const params = new URLSearchParams("foo=bar&baz=qux");
/// params.get("foo"); // "bar"
/// ```
///
/// # Constructor Options
/// - String: Parse as query string
/// - Object: Convert to query string
/// - Array of tuples: Convert to query string
#[derive(Clone, Debug)]
pub struct URLSearchParams {
    params: HashMap<String, Vec<String>>,
}

impl URLSearchParams {
    /// Create a new empty URLSearchParams
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    /// Create URLSearchParams from a query string
    ///
    /// # Arguments
    /// * `query` - The query string to parse (with or without leading '?')
    pub fn from_query(query: &str) -> Self {
        let mut params = URLSearchParams::new();

        let query = if query.starts_with('?') {
            &query[1..]
        } else {
            query
        };

        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            let key = if let Some(k) = parts.get(0) {
                decode_uri_component(k)
            } else {
                continue;
            };
            let value = if parts.len() > 1 {
                decode_uri_component(parts[1])
            } else {
                String::new()
            };
            params
                .params
                .entry(key)
                .or_insert_with(Vec::new)
                .push(value);
        }

        params
    }

    /// Create URLSearchParams from an object
    pub fn from_object(obj: &HashMap<String, String>) -> Self {
        let mut params = URLSearchParams::new();
        for (key, value) in obj {
            params
                .params
                .entry(key.clone())
                .or_insert_with(Vec::new)
                .push(value.clone());
        }
        params
    }

    /// Get the first value for a key
    ///
    /// # Arguments
    /// * `name` - The parameter name
    ///
    /// # Returns
    /// The first value or None if the key doesn't exist
    pub fn get(&self, name: &str) -> Option<String> {
        self.params.get(name).and_then(|v| v.first().cloned())
    }

    /// Get all values for a key
    ///
    /// # Arguments
    /// * `name` - The parameter name
    ///
    /// # Returns
    /// A slice of all values for the key
    pub fn get_all(&self, name: &str) -> Vec<String> {
        self.params.get(name).cloned().unwrap_or_default()
    }

    /// Check if a key exists
    ///
    /// # Arguments
    /// * `name` - The parameter name
    ///
    /// # Returns
    /// true if the key exists
    pub fn has(&self, name: &str) -> bool {
        self.params.contains_key(name)
    }

    /// Set a value for a key (replaces all existing values)
    ///
    /// # Arguments
    /// * `name` - The parameter name
    /// * `value` - The value to set
    pub fn set(&mut self, name: &str, value: &str) {
        self.params
            .insert(name.to_string(), vec![value.to_string()]);
    }

    /// Append a value for a key
    ///
    /// # Arguments
    /// * `name` - The parameter name
    /// * `value` - The value to append
    pub fn append(&mut self, name: &str, value: &str) {
        self.params
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(value.to_string());
    }

    /// Delete a key
    ///
    /// # Arguments
    /// * `name` - The parameter name
    pub fn delete(&mut self, name: &str) {
        self.params.remove(name);
    }

    /// Get the number of parameters
    pub fn len(&self) -> usize {
        self.params.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Get an iterator over the keys
    pub fn keys(&self) -> Vec<String> {
        self.params.keys().cloned().collect()
    }

    /// Convert to query string
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        for (i, (key, values)) in self.params.iter().enumerate() {
            if i > 0 {
                result.push('&');
            }
            for (j, value) in values.iter().enumerate() {
                if j > 0 {
                    result.push('&');
                }
                result.push_str(&encode_uri_component(key));
                result.push('=');
                result.push_str(&encode_uri_component(value));
            }
        }
        result
    }

    /// Get entries as an iterator
    pub fn entries(&self) -> Vec<(String, String)> {
        let mut entries = Vec::new();
        for (key, values) in &self.params {
            for value in values {
                entries.push((key.clone(), value.clone()));
            }
        }
        entries
    }
}

impl Default for URLSearchParams {
    fn default() -> Self {
        Self::new()
    }
}

/// URL implementation
///
/// Provides methods for URL parsing and manipulation.
///
/// # JavaScript API
/// ```javascript
/// const url = new URL("https://example.com:8080/path?query=value#hash");
/// url.href; // "https://example.com:8080/path?query=value#hash"
/// url.hostname; // "example.com"
/// url.port; // "8080"
/// url.pathname; // "/path"
/// url.search; // "?query=value"
/// url.hash; // "#hash"
/// ```
///
/// # Constructor Options
/// - `url`: The URL to parse
/// - `base`: Optional base URL for relative URLs
#[derive(Clone, Debug)]
pub struct Url {
    scheme: String,
    username: String,
    password: String,
    host: String,
    port: Option<u16>,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

impl Url {
    /// Create a new URL from a string
    ///
    /// # Arguments
    /// * `url_str` - The URL string to parse
    ///
    /// # Returns
    /// Result containing the URL or an error message
    pub fn new(url_str: &str) -> Result<Self, String> {
        let url_str = url_str.trim();

        // Parse scheme
        let (scheme, rest) = match url_str.find("://") {
            Some(pos) => {
                let scheme = &url_str[..pos];
                let rest = &url_str[pos + 3..];
                if scheme.is_empty() {
                    return Err("Empty scheme".to_string());
                }
                if !scheme
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '+' || c == '-' || c == '.')
                {
                    return Err("Invalid scheme".to_string());
                }
                (scheme.to_lowercase(), rest)
            }
            None => {
                return Err("Invalid URL: missing scheme".to_string());
            }
        };

        // Check for special schemes that don't require host
        if scheme == "file" {
            return parse_file_url(rest);
        }

        // Parse authority (user:password@host:port)
        let mut username = String::new();
        let mut password = String::new();
        let mut host = String::new();
        let mut port: Option<u16> = None;

        let mut rest = rest;

        // Check for user:password@
        if let Some(at_pos) = rest.find('@') {
            let auth = &rest[..at_pos];
            rest = &rest[at_pos + 1..];

            if let Some(colon_pos) = auth.find(':') {
                username = decode_uri_component(&auth[..colon_pos]);
                password = decode_uri_component(&auth[colon_pos + 1..]);
            } else {
                username = decode_uri_component(auth);
            }
        }

        // Parse host:port
        let (host_port, path_query_fragment) = split_host_port(rest);

        host = host_port.host.to_string();
        if let Some(p) = host_port.port {
            port = Some(p);
        }

        // Parse path, query, and fragment
        let (path, query, fragment) = parse_path_query_fragment(path_query_fragment);

        Ok(Self {
            scheme,
            username,
            password,
            host,
            port,
            path,
            query,
            fragment,
        })
    }

    /// Create a URL with a base URL
    ///
    /// # Arguments
    /// * `url_str` - The URL string to parse
    /// * `base` - The base URL
    ///
    /// # Returns
    /// Result containing the URL or an error message
    pub fn with_base(url_str: &str, base: &str) -> Result<Self, String> {
        let url_str = url_str.trim();

        // If url_str is absolute, parse it directly
        if url_str.contains("://") {
            return Self::new(url_str);
        }

        // Parse base URL
        let base_url = Self::new(base)?;

        // Handle relative URLs
        if url_str.starts_with('/') {
            // Absolute path
            let mut result = base_url.clone();
            result.path = url_str.to_string();
            result.query = None;
            result.fragment = None;
            return Ok(result);
        } else if url_str.starts_with("?") {
            // Query only
            let mut result = base_url.clone();
            result.query = Some(url_str[1..].to_string());
            result.fragment = None;
            return Ok(result);
        } else if url_str.starts_with("#") {
            // Fragment only
            let mut result = base_url.clone();
            result.fragment = Some(url_str[1..].to_string());
            return Ok(result);
        } else {
            // Relative path
            let base_path = if base_url.path.is_empty() {
                "/".to_string()
            } else {
                base_url.path.clone()
            };

            // Find the last slash in the path
            let last_slash = base_path.rfind('/').unwrap_or(0);
            let base_dir = &base_path[..last_slash + 1];

            // Resolve relative path
            let mut path = base_dir.to_string() + url_str;

            // Normalize path (remove . and ..)
            path = normalize_path(&path);

            let mut result = base_url.clone();
            result.path = path;
            result.query = None;
            result.fragment = None;
            return Ok(result);
        }
    }

    /// Get the full URL
    pub fn href(&self) -> String {
        let mut result = String::new();

        result.push_str(&self.scheme);
        result.push_str("://");

        if !self.username.is_empty() || !self.password.is_empty() {
            result.push_str(&encode_uri_component(&self.username));
            if !self.password.is_empty() {
                result.push(':');
                result.push_str(&encode_uri_component(&self.password));
            }
            result.push('@');
        }

        result.push_str(&self.host);

        if let Some(port) = self.port {
            result.push(':');
            result.push_str(&port.to_string());
        }

        result.push_str(&self.path);

        if let Some(query) = &self.query {
            result.push('?');
            result.push_str(query);
        }

        if let Some(fragment) = &self.fragment {
            result.push('#');
            result.push_str(fragment);
        }

        result
    }

    /// Get the protocol scheme
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// Get the username
    pub fn username(&self) -> String {
        self.username.clone()
    }

    /// Set the username
    pub fn set_username(&mut self, username: &str) {
        self.username = username.to_string();
    }

    /// Get the password
    pub fn password(&self) -> String {
        self.password.clone()
    }

    /// Set the password
    pub fn set_password(&mut self, password: &str) {
        self.password = password.to_string();
    }

    /// Get the host
    pub fn host(&self) -> String {
        self.host.clone()
    }

    /// Set the host
    pub fn set_host(&mut self, host: &str) {
        self.host = host.to_string();
    }

    /// Get the port
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Set the port
    pub fn set_port(&mut self, port: Option<u16>) {
        self.port = port;
    }

    /// Get the hostname (without port)
    pub fn hostname(&self) -> String {
        self.host.clone()
    }

    /// Get the pathname
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// Set the pathname
    pub fn set_path(&mut self, path: &str) {
        self.path = path.to_string();
    }

    /// Get the search query string (with leading '?')
    pub fn search(&self) -> String {
        if let Some(query) = &self.query {
            format!("?{}", query)
        } else {
            String::new()
        }
    }

    /// Get the search query string (without leading '?')
    pub fn search_params_string(&self) -> Option<String> {
        self.query.clone()
    }

    /// Set the search query
    pub fn set_search(&mut self, search: &str) {
        let search = search.trim();
        self.query = if search.starts_with('?') {
            Some(search[1..].to_string())
        } else if search.is_empty() {
            None
        } else {
            Some(search.to_string())
        };
    }

    /// Get the hash (with leading '#')
    pub fn hash(&self) -> String {
        if let Some(fragment) = &self.fragment {
            format!("#{}", fragment)
        } else {
            String::new()
        }
    }

    /// Get the hash (without leading '#')
    pub fn fragment(&self) -> Option<String> {
        self.fragment.clone()
    }

    /// Set the hash
    pub fn set_hash(&mut self, hash: &str) {
        let hash = hash.trim();
        self.fragment = if hash.starts_with('#') {
            Some(hash[1..].to_string())
        } else if hash.is_empty() {
            None
        } else {
            Some(hash.to_string())
        };
    }

    /// Get the origin (scheme + host + port)
    pub fn origin(&self) -> String {
        let mut result = format!("{}://{}", self.scheme, self.host);
        if let Some(port) = self.port {
            result.push(':');
            result.push_str(&port.to_string());
        }
        result
    }

    /// Get the URLSearchParams for the query string
    pub fn search_params(&self) -> URLSearchParams {
        if let Some(query) = &self.query {
            URLSearchParams::from_query(query)
        } else {
            URLSearchParams::new()
        }
    }

    /// Check if the URL is absolute
    pub fn is_absolute(&self) -> bool {
        !self.scheme.is_empty()
    }

    /// Get the file URL path
    pub fn to_file_path(&self) -> Option<String> {
        if self.scheme == "file" {
            Some(self.path.clone())
        } else {
            None
        }
    }
}

fn parse_file_url(rest: &str) -> Result<Url, String> {
    let mut path = String::new();
    let mut query = None;
    let mut fragment = None;

    // Skip slashes
    let mut rest = rest;
    while rest.starts_with('/') {
        path.push('/');
        rest = &rest[1..];
    }

    // Find where path ends
    let (path_part, rest) = if let Some(q_pos) = rest.find('?') {
        (&rest[..q_pos], &rest[q_pos..])
    } else if let Some(f_pos) = rest.find('#') {
        (&rest[..f_pos], &rest[f_pos..])
    } else {
        (rest, "")
    };

    path.push_str(path_part);

    if !rest.is_empty() {
        if rest.starts_with('?') {
            let (q, f) = if let Some(f_pos) = rest[1..].find('#') {
                (&rest[1..1 + f_pos], Some(&rest[1 + f_pos + 1..]))
            } else {
                (&rest[1..], None)
            };
            query = Some(q.to_string());
            if let Some(f) = f {
                fragment = Some(f.to_string());
            }
        } else if rest.starts_with('#') {
            fragment = Some(rest[1..].to_string());
        }
    }

    Ok(Url {
        scheme: "file".to_string(),
        username: String::new(),
        password: String::new(),
        host: String::new(),
        port: None,
        path: path,
        query,
        fragment,
    })
}

struct HostPort<'a> {
    host: &'a str,
    port: Option<u16>,
}

fn split_host_port(s: &str) -> (HostPort<'_>, &str) {
    // Look for the first slash, question mark, or hash
    let mut end_pos = s.len();
    let mut slash_pos = None;
    let mut query_pos = None;
    let mut hash_pos = None;

    for (i, c) in s.char_indices() {
        match c {
            '/' if slash_pos.is_none() => {
                slash_pos = Some(i);
                if query_pos.is_some() || hash_pos.is_some() {
                    break;
                }
            }
            '?' if query_pos.is_none() => {
                query_pos = Some(i);
                if slash_pos.is_some() || hash_pos.is_some() {
                    break;
                }
            }
            '#' if hash_pos.is_none() => {
                hash_pos = Some(i);
                if slash_pos.is_some() || query_pos.is_some() {
                    break;
                }
            }
            _ => {}
        }
    }

    let authority_part = if let Some(pos) = [slash_pos, query_pos, hash_pos]
        .iter()
        .filter_map(|&p| p)
        .min()
    {
        end_pos = pos;
        &s[..pos]
    } else {
        &s
    };

    // Parse host:port
    if let Some(colon_pos) = authority_part.rfind(':') {
        let host = &authority_part[..colon_pos];
        let port_str = &authority_part[colon_pos + 1..];
        if port_str.parse::<u16>().is_ok() {
            (
                HostPort {
                    host,
                    port: Some(port_str.parse().unwrap()),
                },
                &s[end_pos..],
            )
        } else {
            (
                HostPort {
                    host: authority_part,
                    port: None,
                },
                &s[end_pos..],
            )
        }
    } else {
        (
            HostPort {
                host: authority_part,
                port: None,
            },
            &s[end_pos..],
        )
    }
}

fn parse_path_query_fragment(s: &str) -> (String, Option<String>, Option<String>) {
    let mut path = String::new();
    let mut query = None;
    let mut fragment = None;

    let mut rest = s;

    // Find path
    let path_end = if let Some(q_pos) = rest.find('?') {
        path.push_str(&rest[..q_pos]);
        &rest[q_pos..]
    } else if let Some(f_pos) = rest.find('#') {
        path.push_str(&rest[..f_pos]);
        &rest[f_pos..]
    } else {
        path.push_str(rest);
        ""
    };

    if path.is_empty() {
        path = "/".to_string();
    }

    if !path_end.is_empty() {
        if path_end.starts_with('?') {
            let (q, f) = if let Some(f_pos) = path_end[1..].find('#') {
                (
                    Some(&path_end[1..1 + f_pos]),
                    Some(&path_end[1 + f_pos + 1..]),
                )
            } else if path_end.len() > 1 {
                (Some(&path_end[1..]), None)
            } else {
                (None, None)
            };
            if let Some(q) = q {
                query = Some(q.to_string());
            }
            if let Some(f) = f {
                fragment = Some(f.to_string());
            }
        } else if path_end.starts_with('#') {
            fragment = Some(path_end[1..].to_string());
        }
    }

    (path, query, fragment)
}

fn normalize_path(path: &str) -> String {
    let mut result = String::new();
    let mut stack: Vec<String> = Vec::new();

    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if !stack.is_empty() && stack.last() != Some(&"..".to_string()) {
                    stack.pop();
                } else {
                    stack.push("..".to_string());
                }
            }
            _ => {
                stack.push(component.to_string());
            }
        }
    }

    for (i, part) in stack.iter().enumerate() {
        if i > 0 || path.starts_with('/') {
            result.push('/');
        }
        result.push_str(part);
    }

    if result.is_empty() {
        result = "/".to_string();
    }

    result
}

fn decode_uri_component(s: &str) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i < s.len() {
        let c = s.as_bytes()[i] as char;
        if c == '%' && i + 2 < s.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(byte as char);
                i += 3;
                continue;
            }
        }
        result.push(c);
        i += 1;
    }
    result
}

fn encode_uri_component(s: &str) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";
    let mut result = String::new();
    for &byte in s.as_bytes() {
        match byte {
            0x41..=0x5A
            | 0x61..=0x7A
            | 0x30..=0x39
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => {
                result.push(byte as char);
            }
            b' ' => {
                result.push('%');
                result.push(HEX_CHARS[(byte >> 4) as usize] as char);
                result.push(HEX_CHARS[(byte & 0x0F) as usize] as char);
            }
            _ => {
                result.push('%');
                result.push(HEX_CHARS[(byte >> 4) as usize] as char);
                result.push(HEX_CHARS[(byte & 0x0F) as usize] as char);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_basic() {
        let url = Url::new("https://example.com:8080/path?query=value#hash").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host(), "example.com");
        assert_eq!(url.port(), Some(8080));
        assert_eq!(url.path(), "/path");
        assert_eq!(url.search(), "?query=value");
        assert_eq!(url.hash(), "#hash");
    }

    #[test]
    fn test_url_without_port() {
        let url = Url::new("https://example.com/path").unwrap();
        assert_eq!(url.port(), None);
        assert_eq!(url.port(), None);
    }

    #[test]
    fn test_url_href() {
        let url = Url::new("https://example.com/path?query=value").unwrap();
        assert!(url.href().starts_with("https://example.com/path"));
    }

    #[test]
    fn test_url_username_password() {
        let url = Url::new("https://user:pass@example.com/path").unwrap();
        assert_eq!(url.username(), "user");
        assert_eq!(url.password(), "pass");
    }

    #[test]
    fn test_url_search_params() {
        let url = Url::new("https://example.com/path?foo=bar&baz=qux").unwrap();
        let params = url.search_params();
        assert_eq!(params.get("foo"), Some("bar".to_string()));
        assert_eq!(params.get("baz"), Some("qux".to_string()));
    }

    #[test]
    fn test_url_origin() {
        let url = Url::new("https://example.com:8080/path").unwrap();
        assert_eq!(url.origin(), "https://example.com:8080");
    }

    #[test]
    fn test_url_file_url() {
        let url = Url::new("file:///tmp/test.txt").unwrap();
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), "/tmp/test.txt");
    }

    #[test]
    fn test_url_setters() {
        let mut url = Url::new("https://example.com/path").unwrap();
        url.set_port(Some(3000));
        url.set_hash("#section");
        assert_eq!(url.port(), Some(3000));
        assert_eq!(url.hash(), "#section");
    }

    #[test]
    fn test_url_with_base() {
        let url = Url::with_base("relative/path", "https://example.com/base/").unwrap();
        assert_eq!(url.host(), "example.com");
        assert!(url.path().contains("relative"));
    }

    #[test]
    fn test_url_invalid() {
        assert!(Url::new("not-a-url").is_err());
        assert!(Url::new("://example.com").is_err());
    }

    #[test]
    fn test_urlsearchparams_from_string() {
        let params = URLSearchParams::from_query("foo=bar&baz=qux");
        assert_eq!(params.get("foo"), Some("bar".to_string()));
        assert_eq!(params.get("baz"), Some("qux".to_string()));
    }

    #[test]
    fn test_urlsearchparams_from_string_with_question_mark() {
        let params = URLSearchParams::from_query("?foo=bar&baz=qux");
        assert_eq!(params.get("foo"), Some("bar".to_string()));
    }

    #[test]
    fn test_urlsearchparams_get_all() {
        let params = URLSearchParams::from_query("foo=bar&foo=baz");
        let all = params.get_all("foo");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], "bar");
        assert_eq!(all[1], "baz");
    }

    #[test]
    fn test_urlsearchparams_has() {
        let params = URLSearchParams::from_query("foo=bar");
        assert!(params.has("foo"));
        assert!(!params.has("baz"));
    }

    #[test]
    fn test_urlsearchparams_set() {
        let mut params = URLSearchParams::new();
        params.set("foo", "bar");
        assert_eq!(params.get("foo"), Some("bar".to_string()));

        params.set("foo", "baz");
        assert_eq!(params.get_all("foo").len(), 1);
    }

    #[test]
    fn test_urlsearchparams_append() {
        let mut params = URLSearchParams::new();
        params.append("foo", "bar");
        params.append("foo", "baz");
        let all = params.get_all("foo");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_urlsearchparams_delete() {
        let mut params = URLSearchParams::from_query("foo=bar&baz=qux");
        assert!(params.has("foo"));
        params.delete("foo");
        assert!(!params.has("foo"));
        assert!(params.has("baz"));
    }

    #[test]
    fn test_urlsearchparams_to_string() {
        let params = URLSearchParams::from_query("foo=bar&baz=qux");
        let s = params.to_string();
        assert!(s.contains("foo=bar"));
        assert!(s.contains("baz=qux"));
    }

    #[test]
    fn test_urlsearchparams_encoding() {
        let params = URLSearchParams::from_query("foo=hello%20world");
        assert_eq!(params.get("foo"), Some("hello world".to_string()));
    }

    #[test]
    fn test_urlsearchparams_empty() {
        let params = URLSearchParams::new();
        assert!(params.is_empty());
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn test_urlsearchparams_keys() {
        let params = URLSearchParams::from_query("foo=bar&baz=qux");
        let keys = params.keys();
        assert!(keys.contains(&"foo".to_string()));
        assert!(keys.contains(&"baz".to_string()));
    }

    #[test]
    fn test_urlsearchparams_entries() {
        let params = URLSearchParams::from_query("foo=bar&foo=baz");
        let entries = params.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], ("foo".to_string(), "bar".to_string()));
        assert_eq!(entries[1], ("foo".to_string(), "baz".to_string()));
    }
}
