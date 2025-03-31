use crate::url::{Url, UrlPath};
use crate::utils::Result;
use crate::{encoder::*, url::Endpoint};

use reqwest::{
    Certificate, Client, Method, Request, StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::collections::HashMap;
use std::fmt::Debug;

const DEFAULT_METHOD: &str = "GET";

pub trait HttpConnectionProfile {
    fn endpoint(&self) -> Option<&Endpoint>;
    fn user(&self) -> Option<&String>;
    fn password(&self) -> Option<&String>;
    fn insecure(&self) -> Option<bool>;
    fn ca_cert(&self) -> Option<&String>;
    fn headers(&self) -> &HashMap<String, String>;
}

pub trait HttpRequestArgs {
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
                .endpoint()
                .expect("Endpoint cannot be empty when building HttpClient")
                .clone(),
            user: args.user().cloned(),
            password: args.password().cloned(),
        }
    }

    pub async fn request(&self, args: &impl HttpRequestArgs) -> Result<HttpResponse> {
        let req = self.build_request(args);
        let res = self.client.execute(req).await?;
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
        let insecure_access = profile.insecure().unwrap_or(false);
        let mut cli_builder = Client::builder()
            .danger_accept_invalid_certs(insecure_access)
            .danger_accept_invalid_hostnames(insecure_access);

        if let Some(ca_cert) = profile.ca_cert() {
            let ca_cert = shellexpand::tilde(&ca_cert).to_string();
            let cert = Certificate::from_pem(&std::fs::read(ca_cert).unwrap()).unwrap();
            cli_builder = cli_builder.use_rustls_tls().add_root_certificate(cert);
        }

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

        let client = cli_builder.build().unwrap();
        client
    }
}
