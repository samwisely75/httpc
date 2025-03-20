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
    pub method: String,
    pub url: String,
    pub body: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub api_key: Option<String>,
    pub insecure: bool,
    pub ca_certs: Option<String>,
    pub content_type: Option<String>,
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

pub async fn send_request(args: RequestArgs) -> Result<Response, StdError> {
    let method = Method::from_str(args.method.as_str()).unwrap();
    let body = args.body.clone().unwrap_or("".to_string());
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

async fn get_client(args: RequestArgs) -> Result<Client, StdError> {
    let mut headers = HeaderMap::new();
    let content_type = args
        .content_type
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
        let auth_value = format!("Bearer {}", args.api_key.unwrap());
        headers.insert("Authorization", HeaderValue::from_str(&auth_value).unwrap());
    } else if args.user.is_some() && args.password.is_some() {
        let auth_str = format!("{}:{}", args.user.unwrap(), args.password.unwrap());
        let auth_value = format!("Basic {}", STANDARD.encode(auth_str));
        headers.insert("Authorization", HeaderValue::from_str(&auth_value).unwrap());
    }

    let mut builder = Client::builder()
        .default_headers(headers)
        .danger_accept_invalid_certs(args.insecure);

    if args.insecure && args.ca_certs.is_some() {
        let cert = tokio::fs::read(args.ca_certs.unwrap()).await?;
        let cert = reqwest::Certificate::from_pem(&cert)?;
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
