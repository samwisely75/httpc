use crate::http::{RequestArgs, Url};
use clap::builder::{OsStringValueParser, TypedValueParser};
use std::{collections::HashMap, ffi::OsString};

pub use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ClapArgs {
    #[clap(help = "HTTP method (GET/POST/PUT/DELETE etc.)")]
    method: String,
    #[clap(value_parser = OsStringValueParser::new().map(|s| Url::parse(s.to_str().unwrap())), help = "URL to send the request")]
    url: Url,
    #[clap(help = "body text to send with the request")]
    body: Option<String>,
    #[clap(short = 'p', long, default_value = "default", help = "profile name")]
    profile: String,
    #[clap(short = 'u', long, help = "username for basic authentication")]
    user: Option<String>,
    #[clap(short = 'w', long, help = "password for basic authentication")]
    password: Option<String>,
    #[clap(short = 'a', long, help = "API key for authentication")]
    api_key: Option<String>,
    #[clap(short = 'r', long, help = "CA certificate PEM file path")]
    ca_cert: Option<String>,
    #[clap(
        short = 'k',
        long,
        help = "Allow insecure server connections when using SSL"
    )]
    insecure: bool,
    #[clap(
        short = 'H',
        long = "header",
        name = "KEY: VALUE",
        help = "HTTP header to send with the request"
    )]
    headers: Vec<String>,

    #[clap(
        short = 'v',
        long,
        help = "Print verbose message",
        default_value = "false"
    )]
    verbose: bool,
}

pub struct CommandLineArgs {
    method: String,
    url: Url,
    body: Option<String>,
    profile: String,
    user: Option<String>,
    password: Option<String>,
    ca_cert: Option<String>,
    insecure: bool,
    headers: HashMap<String, String>,
    verbose: bool,
}

fn vec_to_hashmap(vec: Vec<String>) -> HashMap<String, String> {
    vec.into_iter()
        .map(|s| {
            let parts: Vec<&str> = s.splitn(2, ':').collect();
            if parts.len() != 2 {
                panic!("Invalid header format: {}", s);
            }
            (parts[0].trim().to_string(), parts[1].trim().to_string())
        })
        .collect::<HashMap<String, String>>()
}

impl CommandLineArgs {
    pub fn parse() -> Self {
        let args = ClapArgs::parse();
        Self {
            method: args.method,
            url: args.url,
            body: args.body,
            profile: args.profile,
            user: args.user,
            password: args.password,
            ca_cert: args.ca_cert,
            insecure: args.insecure,
            headers: vec_to_hashmap(args.headers),
            verbose: args.verbose,
        }
    }

    #[allow(dead_code)]
    pub fn parse_from<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let args = ClapArgs::parse_from(itr);
        Self {
            method: args.method,
            url: args.url,
            body: args.body,
            profile: args.profile,
            user: args.user,
            password: args.password,
            ca_cert: args.ca_cert,
            insecure: args.insecure,
            headers: vec_to_hashmap(args.headers),
            verbose: args.verbose,
        }
    }

    pub fn profile(&self) -> &String {
        &self.profile
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }
}

impl RequestArgs for CommandLineArgs {
    fn method(&self) -> Option<&String> {
        Some(&self.method)
    }

    fn url(&self) -> Option<&Url> {
        Some(&self.url)
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

#[cfg(test)]
mod test {
    use super::*;

    const TEST_METHOD: &str = "GET";
    const TEST_URL: &str = "https://example.com";
    const TEST_TEXT: &str = "{ \"query\": { \"match_all\": {} } }";
    const TEST_PROFILE: &str = "default";
    const TEST_USER: &str = "user";
    const TEST_PASSWORD: &str = "password";
    const TEST_CA_CERT: &str = "/path/to/ca_cert.pem";
    const TEST_INSECURE: bool = true;
    const TEST_HEADER_CONTENT_TYPE: &str = "Content-Type: application/json";
    const TEST_HEADER_USER_AGENT: &str = "User-Agent: wiq/0.0.1-SNAPSHOT";

    #[test]
    fn test_cli() {
        use clap::CommandFactory;
        ClapArgs::command().debug_assert()
    }

    #[test]
    fn test_parse_args() {
        let params = vec![
            "http",
            TEST_METHOD,
            TEST_URL,
            TEST_TEXT,
            "-p",
            TEST_PROFILE,
            "-u",
            TEST_USER,
            "-w",
            TEST_PASSWORD,
            "-r",
            TEST_CA_CERT,
            "-k",
            "-H",
            TEST_HEADER_CONTENT_TYPE,
            "-H",
            TEST_HEADER_USER_AGENT,
        ];
        let args = CommandLineArgs::parse_from(params.iter());

        assert_eq!(args.method, TEST_METHOD);
        assert_eq!(args.url().unwrap().to_string(), TEST_URL);
        assert_eq!(args.body, Some(TEST_TEXT.to_string()));
        assert_eq!(args.profile, TEST_PROFILE);
        assert_eq!(args.user, Some(TEST_USER.to_string()));
        assert_eq!(args.password, Some(TEST_PASSWORD.to_string()));
        assert_eq!(args.ca_cert, Some(TEST_CA_CERT.to_string()));
        assert_eq!(args.insecure, TEST_INSECURE);

        assert_eq!(args.headers.len(), 2);
        assert_eq!(
            args.headers["Content-Type"],
            TEST_HEADER_CONTENT_TYPE.split(":").nth(1).unwrap().trim()
        );
        assert_eq!(
            args.headers["User-Agent"],
            TEST_HEADER_USER_AGENT.split(":").nth(1).unwrap().trim()
        );
    }
}
