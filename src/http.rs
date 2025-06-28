use crate::url::{Url, UrlPath};
use crate::utils::Result;
use crate::{decoder::*, url::Endpoint};

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
    pub fn new(args: &impl HttpConnectionProfile) -> Self {
        let client = Self::build_client(args);
        HttpClient {
            client,
            endpoint: args
                .server()
                .expect("Endpoint cannot be empty when building HttpClient")
                .clone(),
            user: args.user().cloned(),
            password: args.password().cloned(),
        }
    }

    pub async fn request(&self, args: &impl HttpRequestArgs) -> Result<HttpResponse> {
        // Build a request
        let req = self.build_request(args);
        // contact the server and receive the response
        let res = self.client.execute(req).await?;

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

    fn build_request(&self, args: &impl HttpRequestArgs) -> Request {
        let default_method = DEFAULT_METHOD.to_string();
        let method_str = args.method().unwrap_or(&default_method);
        let method = Method::from_bytes(method_str.as_bytes()).unwrap();
        let url = Url::new(Some(&self.endpoint), args.url_path()).to_string();

        let mut req_builder = self.client.request(method, url);

        if let Some(body) = args.body() {
            req_builder = req_builder.body(body.to_string());
        }

        if let Some(user) = &self.user {
            req_builder = req_builder.basic_auth(user, self.password.clone());
        }

        req_builder.build().unwrap()
    }

    fn build_client(profile: &impl HttpConnectionProfile) -> Client {
        // insecure access
        let insecure_access = profile.insecure().unwrap_or(false);
        let mut cli_builder = Client::builder()
            .danger_accept_invalid_certs(insecure_access)
            .danger_accept_invalid_hostnames(insecure_access);

        // custom CA certificates
        if let Some(ca_cert) = profile.ca_cert() {
            let ca_cert = shellexpand::tilde(&ca_cert).to_string();
            let cert = Certificate::from_pem(&std::fs::read(ca_cert).unwrap()).unwrap();
            cli_builder = cli_builder.use_rustls_tls().add_root_certificate(cert);
        }

        // default headers
        if !profile.headers().is_empty() {
            let headers = profile
                .headers()
                .iter()
                .map(|(key, value)| {
                    (
                        HeaderName::from_bytes(key.as_bytes()).unwrap(),
                        HeaderValue::from_str(value.as_str()).unwrap(),
                    )
                })
                .collect::<HeaderMap>();
            cli_builder = cli_builder.default_headers(headers);
        }

        // proxy
        if let Some(proxy) = profile.proxy() {
            let proxy_url = proxy.to_string();
            cli_builder = cli_builder.proxy(reqwest::Proxy::all(proxy_url).unwrap());
        }

        cli_builder.build().unwrap()
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
        let client = HttpClient::new(&profile);

        assert_eq!(client.endpoint.scheme(), Some(&"https".to_string()));
        assert_eq!(client.endpoint.host(), "httpbin.org");
        assert!(client.user.is_none());
        assert!(client.password.is_none());
    }

    #[test]
    fn test_http_client_with_auth() {
        let profile = MockProfile::new().with_auth("testuser".to_string(), "testpass".to_string());
        let client = HttpClient::new(&profile);

        assert_eq!(client.user, Some("testuser".to_string()));
        assert_eq!(client.password, Some("testpass".to_string()));
    }

    #[test]
    fn test_build_request_get() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile);
        let request_args = MockRequest::new();

        let request = client.build_request(&request_args);

        assert_eq!(request.method(), &Method::GET);
        assert_eq!(request.url().path(), "/get");
    }

    #[test]
    fn test_build_request_post_with_body() {
        let profile = MockProfile::new();
        let client = HttpClient::new(&profile);
        let request_args = MockRequest::new()
            .with_method("POST")
            .with_body("{\"test\": \"data\"}");

        let request = client.build_request(&request_args);

        assert_eq!(request.method(), &Method::POST);
        assert!(request.body().is_some());
    }

    #[test]
    fn test_build_request_with_custom_headers() {
        let mut headers = HashMap::new();
        headers.insert("x-custom-header".to_string(), "custom-value".to_string());
        headers.insert("authorization".to_string(), "Bearer token123".to_string());

        let profile = MockProfile::new().with_headers(headers.clone());
        let client = HttpClient::new(&profile);
        let request_args = MockRequest::new().with_headers(headers);

        let request = client.build_request(&request_args);

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
}
