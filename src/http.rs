use crate::url::{Url, UrlPath};
use crate::utils::Result;
use crate::{decoder::*, url::Endpoint};

use anyhow::Context;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Certificate, Client, Method, Request, StatusCode,
};
use std::collections::HashMap;
use std::fmt::Debug;

const DEFAULT_METHOD: &str = "GET";

pub trait HttpConnectionProfile: Debug {
    fn server(&self) -> Option<&Endpoint>;
    fn user(&self) -> Option<&String>;
    fn password(&self) -> Option<&String>;
    fn insecure(&self) -> Option<bool>;
    fn ca_cert(&self) -> Option<&String>;
    fn headers(&self) -> &HashMap<String, String>;
    fn proxy(&self) -> Option<&Endpoint>;
}

pub trait HttpRequestArgs: Debug {
    fn method(&self) -> Option<&String>;
    fn url_path(&self) -> Option<&UrlPath>;
    fn body(&self) -> Option<&String>;
    fn headers(&self) -> &HashMap<String, String>;
}

#[derive(Debug)]
pub struct HttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
    json: Option<serde_json::Value>,
}

impl HttpResponse {
    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn json(&self) -> Option<&serde_json::Value> {
        self.json.as_ref()
    }
}

pub struct HttpClient {
    client: Client,
    endpoint: Endpoint,
    user: Option<String>,
    password: Option<String>,
}

impl Debug for HttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("client", &"Client")
            .finish()
    }
}

impl HttpClient {
    pub fn new(args: &impl HttpConnectionProfile) -> Result<Self> {
        let client = Self::build_client(args)?;
        Ok(HttpClient {
            client,
            endpoint: args
                .server()
                .ok_or_else(|| {
                    anyhow::anyhow!("Endpoint cannot be empty when building HttpClient")
                })?
                .clone(),
            user: args.user().cloned(),
            password: args.password().cloned(),
        })
    }

    pub async fn request(&self, args: &impl HttpRequestArgs) -> Result<HttpResponse> {
        // Build a request
        let req = self
            .build_request(args)
            .context("Failed to build HTTP request")?;
        // contact the server and receive the response
        let res = self
            .client
            .execute(req)
            .await
            .context("Failed to execute HTTP request")?;

        // Acquire the response status and headers
        let headers = res.headers().clone();
        let status = res.status();

        // Decode the response body (decompress and decode to UTF-8/SHIFT-JIS)
        let default_encoding = HeaderValue::from_static(ENC_NONE);
        let content_encoding = headers
            .get("content-encoding")
            .unwrap_or(&default_encoding)
            .to_str()?;
        let body_bytes = res.bytes().await?;
        let body_string = decode_bytes(&body_bytes, content_encoding)?;
        let content_type = headers
            .get("content-type")
            .unwrap_or(&default_encoding)
            .to_str()?;
        let json = if content_type.contains("application/json") {
            Some(serde_json::from_str(&body_string)?)
        } else {
            None
        };

        Ok(HttpResponse {
            status,
            headers,
            body: body_string,
            json,
        })
    }

    fn build_request(&self, args: &impl HttpRequestArgs) -> Result<Request> {
        let default_method = DEFAULT_METHOD.to_string();
        let method_str = args.method().unwrap_or(&default_method);
        let method = Method::from_bytes(method_str.as_bytes())
            .with_context(|| format!("Invalid HTTP method '{method_str}'"))?;
        let url = Url::new(Some(&self.endpoint), args.url_path()).to_string();

        let mut req_builder = self.client.request(method, url);

        if let Some(body) = args.body() {
            req_builder = req_builder.body(body.to_string());
        }

        if let Some(user) = &self.user {
            req_builder = req_builder.basic_auth(user, self.password.clone());
        }

        // Add headers from request arguments
        for (key, value) in args.headers() {
            let header_name = HeaderName::from_bytes(key.as_bytes())
                .with_context(|| format!("Invalid header name '{key}'"))?;
            let header_value = HeaderValue::from_str(value.as_str())
                .with_context(|| format!("Invalid header value '{value}' for header '{key}'"))?;
            req_builder = req_builder.header(header_name, header_value);
        }

        req_builder.build().context("Failed to build HTTP request")
    }

    fn build_client(profile: &impl HttpConnectionProfile) -> Result<Client> {
        // insecure access
        let insecure_access = profile.insecure().unwrap_or(false);
        let mut cli_builder = Client::builder()
            .danger_accept_invalid_certs(insecure_access)
            .danger_accept_invalid_hostnames(insecure_access);

        // custom CA certificates
        if let Some(ca_cert) = profile.ca_cert() {
            let ca_cert = shellexpand::tilde(&ca_cert).to_string();
            let cert_data = std::fs::read(&ca_cert)
                .with_context(|| format!("Failed to read CA certificate file '{ca_cert}'"))?;
            let cert = Certificate::from_pem(&cert_data)
                .with_context(|| format!("Failed to parse CA certificate from '{ca_cert}'"))?;
            cli_builder = cli_builder.use_rustls_tls().add_root_certificate(cert);
        }

        // default headers
        if !profile.headers().is_empty() {
            let mut headers = HeaderMap::new();
            for (key, value) in profile.headers() {
                let header_name = HeaderName::from_bytes(key.as_bytes())
                    .with_context(|| format!("Invalid header name '{key}'"))?;
                let header_value = HeaderValue::from_str(value.as_str()).with_context(|| {
                    format!("Invalid header value '{value}' for header '{key}'")
                })?;
                headers.insert(header_name, header_value);
            }
            cli_builder = cli_builder.default_headers(headers);
        }

        // proxy
        if let Some(proxy) = profile.proxy() {
            let proxy_url = proxy.to_string();
            let proxy = reqwest::Proxy::all(&proxy_url)
                .with_context(|| format!("Failed to configure proxy '{proxy_url}'"))?;
            cli_builder = cli_builder.proxy(proxy);
        }

        cli_builder.build().context("Failed to build HTTP client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::url::{Endpoint, UrlPath};
    use std::collections::HashMap;

    #[derive(Debug)]
    struct MockProfile {
        server: Option<Endpoint>,
        user: Option<String>,
        password: Option<String>,
        insecure: Option<bool>,
        ca_cert: Option<String>,
        headers: HashMap<String, String>,
        proxy: Option<Endpoint>,
    }

    impl MockProfile {
        fn new() -> Self {
            Self {
                server: match Endpoint::parse("https://httpbin.org") {
                    Ok(endpoint) => Some(endpoint),
                    Err(_) => panic!("Failed to parse test endpoint"),
                },
                user: None,
                password: None,
                insecure: None,
                ca_cert: None,
                headers: HashMap::new(),
                proxy: None,
            }
        }

        fn with_auth(mut self, user: String, password: String) -> Self {
            self.user = Some(user);
            self.password = Some(password);
            self
        }

        fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
            self.headers = headers;
            self
        }

        #[allow(dead_code)]
        fn with_insecure(mut self, insecure: bool) -> Self {
            self.insecure = Some(insecure);
            self
        }

        #[allow(dead_code)]
        fn with_proxy(mut self, proxy: Endpoint) -> Self {
            self.proxy = Some(proxy);
            self
        }

        #[allow(dead_code)]
        fn with_ca_cert(mut self, ca_cert: String) -> Self {
            self.ca_cert = Some(ca_cert);
            self
        }
    }

    impl HttpConnectionProfile for MockProfile {
        fn server(&self) -> Option<&Endpoint> {
            self.server.as_ref()
        }

        fn user(&self) -> Option<&String> {
            self.user.as_ref()
        }

        fn password(&self) -> Option<&String> {
            self.password.as_ref()
        }

        fn insecure(&self) -> Option<bool> {
            self.insecure
        }

        fn ca_cert(&self) -> Option<&String> {
            self.ca_cert.as_ref()
        }

        fn headers(&self) -> &HashMap<String, String> {
            &self.headers
        }

        fn proxy(&self) -> Option<&Endpoint> {
            self.proxy.as_ref()
        }
    }

    #[derive(Debug)]
    struct MockRequest {
        method: Option<String>,
        url_path: Option<UrlPath>,
        body: Option<String>,
        headers: HashMap<String, String>,
    }

    impl MockRequest {
        fn new() -> Self {
            Self {
                method: Some("GET".to_string()),
                url_path: Some(UrlPath::new("/get".to_string(), None)),
                body: None,
                headers: HashMap::new(),
            }
        }

        fn with_method(mut self, method: &str) -> Self {
            self.method = Some(method.to_string());
            self
        }

        fn with_body(mut self, body: &str) -> Self {
            self.body = Some(body.to_string());
            self
        }

        fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
            self.headers = headers;
            self
        }
    }

    impl HttpRequestArgs for MockRequest {
        fn method(&self) -> Option<&String> {
            self.method.as_ref()
        }

        fn url_path(&self) -> Option<&UrlPath> {
            self.url_path.as_ref()
        }

        fn body(&self) -> Option<&String> {
            self.body.as_ref()
        }

        fn headers(&self) -> &HashMap<String, String> {
            &self.headers
        }
    }

    #[test]
    fn test_http_client_creation() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile).unwrap();
        assert_eq!(client.endpoint.scheme(), Some(&"https".to_string()));
        assert_eq!(client.endpoint.host(), "httpbin.org");
        assert!(client.user.is_none());
        assert!(client.password.is_none());
    }

    #[test]
    fn test_http_client_with_auth() {
        let profile = MockProfile::new().with_auth("testuser".to_string(), "testpass".to_string());
        let client = HttpClient::new(&profile).unwrap();

        assert_eq!(client.user, Some("testuser".to_string()));
        assert_eq!(client.password, Some("testpass".to_string()));
    }

    #[test]
    fn test_build_request_get() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile).unwrap();
        let request_args = MockRequest::new();

        let request = client.build_request(&request_args).unwrap();

        assert_eq!(request.method(), &Method::GET);
        assert_eq!(request.url().path(), "/get");
    }

    #[test]
    fn test_build_request_post_with_body() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile).unwrap();
        let request_args = MockRequest::new()
            .with_method("POST")
            .with_body("{\"test\": \"data\"}");

        let request = client.build_request(&request_args).unwrap();

        assert_eq!(request.method(), &Method::POST);
        assert!(request.body().is_some());
    }

    #[test]
    fn test_build_request_with_custom_headers() {
        let mut headers = HashMap::new();
        headers.insert("x-custom-header".to_string(), "custom-value".to_string());
        headers.insert("authorization".to_string(), "Bearer token123".to_string());

        let profile = MockProfile::new().with_headers(headers.clone());
        let client = HttpClient::new(&profile).unwrap();
        let request_args = MockRequest::new().with_headers(headers);

        let request = client.build_request(&request_args).unwrap();

        assert!(request.headers().get("x-custom-header").is_some());
        assert!(request.headers().get("authorization").is_some());
    }

    #[test]
    fn test_response_methods() {
        let response = HttpResponse {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: "test body".to_string(),
            json: Some(serde_json::json!({"test": "value"})),
        };

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.body(), "test body");
        assert!(response.json().is_some());
        assert_eq!(response.json().unwrap()["test"], "value");
    }

    #[test]
    fn test_default_method() {
        assert_eq!(DEFAULT_METHOD, "GET");
    }

    #[test]
    fn test_http_response_creation() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());

        let response = HttpResponse {
            status: StatusCode::OK,
            headers: headers.clone(),
            body: "test response".to_string(),
            json: Some(serde_json::json!({"key": "value"})),
        };

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.body(), "test response");
        assert_eq!(response.headers(), &headers);
        assert!(response.json().is_some());
        assert_eq!(response.json().unwrap()["key"], "value");
    }

    #[test]
    fn test_http_response_without_json() {
        let response = HttpResponse {
            status: StatusCode::NOT_FOUND,
            headers: HeaderMap::new(),
            body: "Not found".to_string(),
            json: None,
        };

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.body(), "Not found");
        assert!(response.json().is_none());
    }

    #[test]
    fn test_build_client_with_insecure() {
        let profile = MockProfile::new().with_insecure(true);

        let client = HttpClient::new(&profile).unwrap();

        // We can't easily test the internal client configuration,
        // but we can verify the client was created successfully
        assert_eq!(client.endpoint.scheme(), Some(&"https".to_string()));
        assert_eq!(client.endpoint.host(), "httpbin.org");
    }

    #[test]
    fn test_build_client_with_proxy() {
        let proxy_endpoint = Endpoint::parse("http://proxy.example.com:8080").unwrap();
        let profile = MockProfile::new().with_proxy(proxy_endpoint);

        let client = HttpClient::new(&profile).unwrap();

        // Verify client creation succeeds with proxy configuration
        assert_eq!(client.endpoint.host(), "httpbin.org");
    }

    #[test]
    fn test_build_request_with_auth() {
        let profile = MockProfile::new().with_auth("testuser".to_string(), "testpass".to_string());
        let client = HttpClient::new(&profile).unwrap();
        let request_args = MockRequest::new();

        let request = client.build_request(&request_args).unwrap();

        // Basic auth should be present in the request
        assert!(request.headers().get("authorization").is_some());
    }

    #[test]
    fn test_build_request_different_methods() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile).unwrap();

        let methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"];

        for method in &methods {
            let request_args = MockRequest::new().with_method(method);
            let request = client.build_request(&request_args).unwrap();

            let expected_method = reqwest::Method::from_bytes(method.as_bytes()).unwrap();
            assert_eq!(request.method(), &expected_method);
        }
    }

    #[test]
    fn test_build_request_with_empty_body() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile).unwrap();
        let request_args = MockRequest::new().with_body("");

        let request = client.build_request(&request_args).unwrap();

        // Empty body should still be included
        assert!(request.body().is_some());
    }

    #[test]
    fn test_build_request_complex_headers() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile).unwrap();

        let mut headers = HashMap::new();
        headers.insert(
            "content-type".to_string(),
            "application/json; charset=utf-8".to_string(),
        );
        headers.insert(
            "accept-encoding".to_string(),
            "gzip, deflate, br".to_string(),
        );
        headers.insert(
            "x-custom-header".to_string(),
            "custom-value-with-special-chars!@#$".to_string(),
        );

        let request_args = MockRequest::new().with_headers(headers);
        let request = client.build_request(&request_args).unwrap();

        assert!(request.headers().get("content-type").is_some());
        assert!(request.headers().get("accept-encoding").is_some());
        assert!(request.headers().get("x-custom-header").is_some());
    }

    #[test]
    fn test_mock_profile_builder_pattern() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        let profile = MockProfile::new()
            .with_auth("user".to_string(), "pass".to_string())
            .with_headers(headers.clone());

        assert_eq!(profile.user(), Some(&"user".to_string()));
        assert_eq!(profile.password(), Some(&"pass".to_string()));
        assert_eq!(profile.headers(), &headers);
    }

    #[test]
    fn test_http_client_debug() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile).unwrap();

        let debug_string = format!("{client:?}");
        assert!(debug_string.contains("HttpClient"));
    }

    #[test]
    fn test_build_request_url_construction() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile).unwrap();

        // Test different URL paths
        let test_cases = vec![
            ("/api/v1/users", None, "https://httpbin.org/api/v1/users"),
            (
                "/search",
                Some("q=rust".to_string()),
                "https://httpbin.org/search?q=rust",
            ),
            ("", None, "https://httpbin.org/"),
        ];

        for (path, query, expected_url) in test_cases {
            let url_path = crate::url::UrlPath::new(path.to_string(), query);
            let mut request_args = MockRequest::new();
            request_args.url_path = Some(url_path);

            let request = client.build_request(&request_args).unwrap();
            assert_eq!(request.url().as_str(), expected_url);
        }
    }

    #[test]
    fn test_error_status_codes() {
        let error_responses = vec![
            (StatusCode::BAD_REQUEST, "400 Bad Request"),
            (StatusCode::UNAUTHORIZED, "401 Unauthorized"),
            (StatusCode::FORBIDDEN, "403 Forbidden"),
            (StatusCode::NOT_FOUND, "404 Not Found"),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "500 Internal Server Error",
            ),
        ];

        for (status, expected_body) in error_responses {
            let response = HttpResponse {
                status,
                headers: HeaderMap::new(),
                body: expected_body.to_string(),
                json: None,
            };

            assert_eq!(response.status(), status);
            assert!(response.body().contains(&status.as_u16().to_string()));
        }
    }
}
