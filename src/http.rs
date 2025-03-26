use base64::{Engine as _, engine::general_purpose::STANDARD};
use flate2::read::{DeflateDecoder, GzDecoder};
use reqwest::{
    Certificate, Client, Method, StatusCode,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::{
    collections::HashMap,
    io::Read,
    str::{self, FromStr},
};

use crate::utils::Result;

const ENC_NONE: &str = "plaintext";
const ENC_GZIP: &str = "gzip";
const ENC_DEFLATE: &str = "deflate";
const ENC_ZSTD: &str = "zstd";

#[derive(Debug)]
pub struct RequestArgs {
    method: String,
    url: String,
    body: Option<String>,
    user: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
    insecure: bool,
    ca_certs: Option<String>,
    pub headers: HashMap<String, String>,
}

impl RequestArgs {
    pub fn new(
        method: String,
        url: String,
        body: Option<String>,
        user: Option<String>,
        password: Option<String>,
        api_key: Option<String>,
        insecure: bool,
        ca_certs: Option<String>,
        headers: HashMap<String, String>,
    ) -> Self {
        RequestArgs {
            method,
            url,
            body,
            user,
            password,
            api_key,
            insecure,
            ca_certs,
            headers,
        }
    }
    pub fn method(&self) -> String {
        self.method.clone()
    }
    pub fn url(&self) -> String {
        self.url.clone()
    }
    pub fn body(&self) -> Option<String> {
        self.body.clone()
    }
    pub fn user(&self) -> Option<String> {
        self.user.clone()
    }
    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }
    pub fn api_key(&self) -> Option<String> {
        self.api_key.clone()
    }
    pub fn insecure(&self) -> bool {
        self.insecure
    }
    pub fn ca_certs(&self) -> Option<String> {
        self.ca_certs.clone()
    }
}
#[derive(Debug)]
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl Response {
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

pub async fn send_request(args: &RequestArgs) -> Result<Response> {
    let method = Method::from_str(args.method.as_str()).unwrap();
    let body = args.body().clone().unwrap_or("".to_string());
    let url = args.url.clone();

    let res = get_client(args)?
        .request(method, url)
        .body(body)
        .send()
        .await
        .unwrap();
    // .await?;
    let headers = res.headers().clone();
    let status = res.status();
    let default_encooding = HeaderValue::from_static(ENC_NONE);
    let encoding = headers
        .get("Content-Encoding")
        .unwrap_or(&default_encooding)
        .to_str()
        .unwrap();
    let body_bytes = res.bytes().await.unwrap();
    let body_str = match encoding {
        ENC_GZIP => decode_gzip(&body_bytes)?,
        ENC_DEFLATE => decode_deflate(&body_bytes)?,
        ENC_ZSTD => decode_zstd(&body_bytes)?,
        ENC_NONE => String::from_utf8(body_bytes.to_vec())?,
        _ => return Err("Unsupported encoding".into()),
    };

    Ok(Response {
        status,
        headers,
        body: body_str,
    })
}

fn get_request_headers(args: &RequestArgs) -> HeaderMap {
    let mut headers = args
        .headers
        .iter()
        .map(|(key, value)| {
            (
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            )
        })
        .collect::<HeaderMap>();

    if args.api_key.is_some() {
        let auth_value = format!("ApiKey {}", args.api_key().unwrap());
        headers.insert("Authorization", HeaderValue::from_str(&auth_value).unwrap());
    } else if args.user().is_some() && args.password().is_some() {
        let auth_str = format!("{}:{}", args.user().unwrap(), args.password().unwrap());
        let auth_value = format!("Basic {}", STANDARD.encode(auth_str));
        headers.insert("Authorization", HeaderValue::from_str(&auth_value).unwrap());
    }

    headers
}

fn get_cert(cert_path: &str) -> Result<Certificate> {
    let p = shellexpand::tilde(cert_path).to_string();
    let cert = std::fs::read(p).unwrap();
    let cert = Certificate::from_pem(&cert)?;
    Ok(cert)
}

fn get_client(args: &RequestArgs) -> Result<Client> {
    let headers = get_request_headers(args);

    let builder = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .danger_accept_invalid_certs(args.insecure())
        .tls_info(true)
        .connection_verbose(true);

    let client = if !args.insecure() && args.ca_certs().is_some() {
        let cert_path = args.ca_certs().unwrap();
        let cert = get_cert(&cert_path)?;
        builder
            .add_root_certificate(cert)
            .use_rustls_tls()
            .build()?
    } else {
        builder.build()?
    };

    Ok(client)
}

fn decode_gzip(data: &[u8]) -> Result<String> {
    let mut decoder = GzDecoder::new(data);
    let mut decoded_data = Vec::new();
    decoder.read_to_end(&mut decoded_data)?;
    Ok(str::from_utf8(&decoded_data)?.to_string())
}

fn decode_deflate(data: &[u8]) -> Result<String> {
    let mut decoder = DeflateDecoder::new(data);
    let mut decoded_data = Vec::new();
    decoder.read_to_end(&mut decoded_data)?;
    Ok(str::from_utf8(&decoded_data)?.to_string())
}

fn decode_zstd(data: &[u8]) -> Result<String> {
    let decoded_data = zstd::decode_all(data)?;
    Ok(str::from_utf8(&decoded_data)?.to_string())
}

#[cfg(test)]
mod test {
    use super::*;

    const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
    const DEFAULT_ACCEPT: &str = "*/*";
    const DEFAULT_ACCEPT_LANG: &str = "ja";
    const DEFAULT_ACCEPT_ENC: &str = "{ENC_GZIP}, {ENC_DEFLATE}, {ENC_ZSTD}";

    #[test]
    fn test_decode_gzip() {
        let data = vec![
            31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 43, 73, 45, 46, 137, 55, 52, 50, 6, 0, 21, 191, 53,
            241, 8, 0, 0, 0,
        ];
        let result = decode_gzip(&data).unwrap();
        assert_eq!(result, "test_123");
    }

    #[test]
    fn test_decode_deflate() {
        let data = vec![43, 73, 45, 46, 137, 55, 52, 50, 6, 0];
        let result = decode_deflate(&data).unwrap();
        assert_eq!(result, "test_123");
    }

    #[test]
    fn test_decode_zstd() {
        let data = vec![
            40, 181, 47, 253, 0, 88, 65, 0, 0, 116, 101, 115, 116, 95, 49, 50, 51,
        ];
        let result = decode_zstd(&data).unwrap();
        assert_eq!(result, "test_123");
    }

    #[test]
    fn test_get_request_headers() -> Result<()> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("User-Agent".to_string(), DEFAULT_USER_AGENT.to_string());
        headers.insert("Accept".to_string(), DEFAULT_ACCEPT.to_string());
        headers.insert(
            "Accept-Encoding".to_string(),
            DEFAULT_ACCEPT_ENC.to_string(),
        );
        headers.insert(
            "Accept-Language".to_string(),
            DEFAULT_ACCEPT_LANG.to_string(),
        );

        let args = RequestArgs {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            body: None,
            user: Some("admin".to_string()),
            password: Some("Password_123".to_string()),
            api_key: None,
            insecure: false,
            ca_certs: None,
            headers: headers,
        };

        let headers = get_request_headers(&args);

        assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
        assert_eq!(headers.get("User-Agent").unwrap(), DEFAULT_USER_AGENT);
        assert_eq!(headers.get("Accept").unwrap(), DEFAULT_ACCEPT);
        assert_eq!(headers.get("Accept-Encoding").unwrap(), DEFAULT_ACCEPT_ENC);
        assert_eq!(headers.get("Accept-Language").unwrap(), DEFAULT_ACCEPT_LANG);
        assert_eq!(
            headers.get("Authorization").unwrap(),
            "Basic YWRtaW46UGFzc3dvcmRfMTIz"
        );

        Ok(())
    }
}
