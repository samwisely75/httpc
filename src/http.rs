use regex::Regex;
use reqwest::{
    Certificate, Client, Method, RequestBuilder, StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::collections::HashMap;
use std::fmt::Debug;

use crate::encoder::*;
use crate::utils::Result;

const DEFAULT_METHOD: &str = "GET";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    scheme: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    path: Option<String>,
    query: Option<String>,
}

impl Url {
    pub fn parse(s: &str) -> Self {
        let re = Regex::new(r"^(?P<scheme>[^:\/]+)?(:\/\/)?(?P<host>[^:\/\?]+)?(:(?P<port>\d+))?(?P<path>[^\?]*)(\?(?P<query>.*))?$").unwrap();
        let caps = re.captures(s).unwrap();

        let url = Url {
            scheme: caps.name("scheme").map(|m| m.as_str().to_string()),
            host: caps.name("host").map(|m| m.as_str().to_string()),
            port: caps.name("port").map(|m| m.as_str().parse::<u16>().unwrap()),
            path: caps.name("path").map(|m| m.as_str().to_string()),
            query: caps.name("query").map(|m| m.as_str().to_string()),
        };
        url
    }

    pub fn merge(&mut self, other: &Url) -> &mut Self {
        if other.scheme.is_some() {
            self.scheme = other.scheme.clone();
        }
        if other.host.is_some() {
            self.host = other.host.clone();
        }
        if other.port.is_some() {
            self.port = other.port.clone();
        }
        if other.path.is_some() {
            self.path = other.path.clone();
        }
        if other.query.is_some() {
            self.query = other.query.clone();
        }
        self
    }

    pub fn scheme(&self) -> Option<&String> {
        self.scheme.as_ref()
    }

    #[allow(dead_code)]
    pub fn host(&self) -> Option<&String> {
        self.host.as_ref()   
    }

    #[allow(dead_code)]
    pub fn port(&self) -> Option<&u16> {
        self.port.as_ref()
    }

    #[allow(dead_code)]
    pub fn path(&self) -> Option<&String> {
        self.path.as_ref()
    }

    #[allow(dead_code)]
    pub fn query(&self) -> Option<&String> {
        self.query.as_ref()
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut url: String = if self.scheme.is_some() && self.host.is_some() {
            format!(
                "{}://{}",
                self.scheme.as_ref().unwrap(),
                self.host.as_ref().unwrap()
            )
        } else {
            "".to_string()
        };

        if let Some(port) = &self.port {
            url.push_str(&format!(":{}", port));
        }

        url.push_str(&self.path.as_ref().unwrap_or(&"".to_string()));

        if self.query.is_some() {
            url.push_str("?");
            url.push_str(&self.query.as_ref().unwrap_or(&"".to_string()));
        }

        write!(f, "{}", url)
    }
}

pub trait RequestArgs {
    fn method(&self) -> Option<&String>;
    fn url(&self) -> Option<&Url>;
    fn body(&self) -> Option<&String>;
    fn user(&self) -> Option<&String>;
    fn password(&self) -> Option<&String>;
    fn insecure(&self) -> bool;
    fn ca_cert(&self) -> Option<&String>;
    fn headers(&self) -> &HashMap<String, String>;
}

#[derive(Debug)]
pub struct HttpRequest {
    method: Option<String>,
    url: Option<Url>,
    body: Option<String>,
    user: Option<String>,
    password: Option<String>,
    insecure: bool,
    ca_cert: Option<String>,
    headers: HashMap<String, String>,
}

impl Clone for HttpRequest {
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            url: self.url.clone(),
            body: self.body.clone(),
            user: self.user.clone(),
            password: self.password.clone(),
            insecure: self.insecure.clone(),
            ca_cert: self.ca_cert.clone(),
            headers: self.headers.clone(),
        }
    }
}

impl RequestArgs for HttpRequest {
    fn method(&self) -> Option<&String> {
        self.method.as_ref()
    }

    fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    fn body(&self) -> Option<&String> {
        self.body.as_ref()
    }

    fn user(&self) -> Option<&String> {
        self.user.as_ref()
    }

    fn password(&self) -> Option<&String> {
        self.password.as_ref()
    }

    fn insecure(&self) -> bool {
        self.insecure
    }

    fn ca_cert(&self) -> Option<&String> {
        self.ca_cert.as_ref()
    }

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

impl HttpRequest {
    pub fn from<T>(args: &T) -> Self
    where
        T: RequestArgs,
    {
        Self {
            method: args.method().map(|s| s.to_string()),
            url: args.url().map(|u| Url::parse(u.to_string().as_str())),
            body: args.body().map(|s| s.to_string()),
            user: args.user().map(|s| s.to_string()),
            password: args.password().map(|s| s.to_string()),
            insecure: args.insecure(),
            ca_cert: args.ca_cert().map(|s| s.to_string()),
            headers: args.headers().clone(),
        }
    }

    pub fn merge<T>(&mut self, args: &T) -> &mut Self
    where
        T: RequestArgs,
    {
        if let Some(url) = args.url() {
            if self.url.is_none() {
                self.url = Some(Url::parse(url.to_string().as_str()));
            } else {
                self.url.as_mut().unwrap().merge(url);
            }
        }

        if let Some(method) = args.method() {
            self.method = Some(method.to_string());
        }

        if let Some(body) = args.body() {
            self.body = Some(body.to_string());
        }

        if let Some(user) = args.user() {
            self.user = Some(user.to_string());
        }

        if let Some(password) = args.password() {
            self.password = Some(password.to_string());
        }

        if let Some(ca_cert) = args.ca_cert() {
            self.ca_cert = Some(ca_cert.to_string());
        }

        self.insecure = args.insecure();

        args.headers().iter().for_each(|(key, value)| {
            self.headers.insert(key.to_string(), value.to_string());
        });

        self
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
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
}

pub struct HttpClient;

impl Debug for HttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("client", &"Client")
            .finish()
    }
}

impl HttpClient {
    pub async fn request<T>(args: &T) -> Result<HttpResponse>
    where
        T: RequestArgs,
    {
        let req = HttpClient::get_request(args);
        let res = req.send().await?;
        let headers = res.headers().clone();
        let status = res.status();
        let body_bytes = res.bytes().await?;
        let default_encoding = HeaderValue::from_static(ENC_NONE);
        let content_encoding = headers
            .get("content-encoding")
            .unwrap_or(&default_encoding)
            .to_str()?;
        let body = match content_encoding {
            ENC_NONE => String::from_utf8(body_bytes.to_vec())?,
            ENC_GZIP => decode_gzip(&body_bytes)?,
            ENC_DEFLATE => decode_deflate(&body_bytes)?,
            ENC_ZSTD => decode_zstd(&body_bytes)?,
            _ => return Err("Unsupported encoding".into()),
        };

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }

    fn get_request<T>(args: &T) -> RequestBuilder
    where
        T: RequestArgs,
    {
        let mut builder = Client::builder()
            .danger_accept_invalid_certs(args.insecure())
            .danger_accept_invalid_hostnames(args.insecure());

        if let Some(ca_cert) = args.ca_cert() {
            let ca_cert = shellexpand::tilde(&ca_cert).to_string();
            let cert = Certificate::from_pem(&std::fs::read(ca_cert).unwrap()).unwrap();
            builder = builder.use_rustls_tls().add_root_certificate(cert);
        }

        let client = builder.build().unwrap();
        let default_method = DEFAULT_METHOD.to_string();
        let method_str = args.method().unwrap_or(&default_method);
        let mut req = client.request(
            Method::from_bytes(method_str.as_bytes()).unwrap(),
            args.url().unwrap().to_string(),
        );

        if let Some(body) = args.body() {
            req = req.body(body.to_string());
        }

        if let Some(user) = args.user() {
            req = req.basic_auth(user, args.password());
        }

        if !args.headers().is_empty() {
            let headers = args
                .headers()
                .iter()
                .map(|(key, value)| {
                    (
                        HeaderName::from_bytes(key.as_bytes()).unwrap(),
                        HeaderValue::from_str(value.as_str()).unwrap(),
                    )
                })
                .collect::<HeaderMap>();
            req = req.headers(headers);
        }

        req
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_url_full() {
        let url = Url::parse("http://example.com:8080/path/to/resource?query=string");
        assert_eq!(url.scheme, Some("http".to_string()));
        assert_eq!(url.host, Some("example.com".to_string()));
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, Some("/path/to/resource".to_string()));
        assert_eq!(url.query, Some("query=string".to_string()));
    }

    #[test]
    fn test_url_relative() {
        let url = Url::parse("/path/to/resource?query=string");
        assert_eq!(url.scheme, None);
        assert_eq!(url.host, None);
        assert_eq!(url.port, None);
        assert_eq!(url.path, Some("/path/to/resource".to_string()));
        assert_eq!(url.query, Some("query=string".to_string()));
    }

    #[test]
    fn test_url_merge() {
        let mut url = Url::parse("https://example.com:9999/");
        let url2 = Url::parse("/path/to/resource?query=string");
        url.merge(&url2);
        assert_eq!(url.scheme, Some("https".to_string()));
        assert_eq!(url.host, Some("example.com".to_string()));
        assert_eq!(url.port, Some(9999));
        assert_eq!(url.path, Some("/path/to/resource".to_string()));
        assert_eq!(url.query, Some("query=string".to_string()));
    }
}
