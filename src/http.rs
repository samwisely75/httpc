use base64::{Engine as _, engine::general_purpose::STANDARD};
use flate2::read::{DeflateDecoder, GzDecoder};
use reqwest::{
    Client, Method, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use std::{
    io::Read,
    str::{self, FromStr},
};
use zstd;

type StdError = Box<dyn std::error::Error>;

const ENC_NONE: &str = "plaintext";
const ENC_GZIP: &str = "gzip";
const ENC_DEFLATE: &str = "deflate";
const ENC_ZSTD: &str = "zstd";

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
const DEFAULT_CONTENT_TYPE: &str = "text/html; charset=UTF-8";
const DEFAULT_ACCEPT: &str = "*/*";
const DEFAULT_ACCEPT_LANG: &str = "ja";
const DEFAULT_ACCEPT_ENC: &str = "{ENC_GZIP}, {ENC_DEFLATE}, {ENC_ZSTD}";

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
    content_type: Option<String>,
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
        content_type: Option<String>,
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
            content_type,
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
    pub fn content_type(&self) -> Option<String> {
        self.content_type.clone()
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

pub async fn send_request(args: &RequestArgs) -> Result<Response, StdError> {
    let method = Method::from_str(args.method.as_str()).unwrap();
    let body = args.body().clone().unwrap_or("".to_string());
    let url = args.url.clone();

    let res = get_client(args)
        .await?
        .request(method, url)
        .body(body)
        .send()
        .await?;
    let headers = res.headers().clone();
    let status = res.status();
    let default_encooding = HeaderValue::from_static(ENC_NONE);
    let encoding = headers
        .get("Content-Encoding")
        .unwrap_or(&default_encooding)
        .to_str()
        .unwrap();
    let body_bytes = res.bytes().await?;
    let body_str = match encoding {
        ENC_GZIP => decode_gzip(&body_bytes)?,
        ENC_DEFLATE => decode_deflate(&body_bytes)?,
        ENC_ZSTD => decode_zstd(&body_bytes)?,
        ENC_NONE => String::from_utf8(body_bytes.to_vec())?,
        _ => return Err("Unsupported encoding".into()),
    };

    Ok(Response {
        status: status,
        headers: headers,
        body: body_str,
    })
}

fn get_client_headers(args: &RequestArgs) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let content_type = args
        .content_type()
        .unwrap_or(DEFAULT_CONTENT_TYPE.to_string());
    headers.insert(
        "Content-Type",
        HeaderValue::from_str(&content_type).unwrap(),
    );
    headers.insert("User-Agent", HeaderValue::from_static(DEFAULT_USER_AGENT));
    headers.insert("Accept", HeaderValue::from_static(DEFAULT_ACCEPT));
    headers.insert(
        "Accept-Encoding",
        HeaderValue::from_static(DEFAULT_ACCEPT_ENC),
    );
    headers.insert(
        "Accept-Language",
        HeaderValue::from_static(DEFAULT_ACCEPT_LANG),
    );

    if args.api_key.is_some() {
        let auth_value = format!("Bearer {}", args.api_key().unwrap());
        headers.insert("Authorization", HeaderValue::from_str(&auth_value).unwrap());
    } else if args.user().is_some() && args.password().is_some() {
        let auth_str = format!("{}:{}", args.user().unwrap(), args.password().unwrap());
        let auth_value = format!("Basic {}", STANDARD.encode(auth_str));
        headers.insert("Authorization", HeaderValue::from_str(&auth_value).unwrap());
    }

    headers
}

async fn get_cert(cert_path: &str) -> Result<reqwest::Certificate, StdError> {
    let cert = tokio::fs::read(cert_path).await?;
    let cert = reqwest::Certificate::from_pem(&cert)?;
    Ok(cert)
}

async fn get_client(args: &RequestArgs) -> Result<Client, StdError> {
    let headers = get_client_headers(args);
    let mut builder = Client::builder()
        .default_headers(headers)
        .danger_accept_invalid_certs(args.insecure());

    if args.insecure() && args.ca_certs().is_some() {
        let cert_path = args.ca_certs().unwrap();
        let cert = get_cert(&cert_path).await?;
        builder = builder.add_root_certificate(cert);
    }

    let client = builder.build()?;
    Ok(client)
}

fn decode_gzip(data: &[u8]) -> Result<String, StdError> {
    let mut decoder = GzDecoder::new(data);
    let mut decoded_data = Vec::new();
    decoder.read_to_end(&mut decoded_data)?;
    Ok(str::from_utf8(&decoded_data)?.to_string())
}

fn decode_deflate(data: &[u8]) -> Result<String, StdError> {
    let mut decoder = DeflateDecoder::new(data);
    let mut decoded_data = Vec::new();
    decoder.read_to_end(&mut decoded_data)?;
    Ok(str::from_utf8(&decoded_data)?.to_string())
}

fn decode_zstd(data: &[u8]) -> Result<String, StdError> {
    let decoded_data = zstd::decode_all(data)?;
    Ok(str::from_utf8(&decoded_data)?.to_string())
}

#[cfg(test)]
mod test {
    use super::*;

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
    fn test_get_client_headers() -> Result<(), StdError> {
        let args = RequestArgs {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            body: None,
            content_type: Some("application/json".to_string()),
            user: Some("admin".to_string()),
            password: Some("Password_123".to_string()),
            api_key: None,
            insecure: false,
            ca_certs: None,
        };

        let headers = get_client_headers(&args);

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
