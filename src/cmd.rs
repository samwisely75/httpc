use std::{collections::HashMap, ffi::OsString};

pub use clap::Parser;
use clap::builder::{OsStringValueParser, TypedValueParser};

use crate::http::{HttpConnectionProfile, HttpRequestArgs};
use crate::url::{Endpoint, Url, UrlPath};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ClapArgs {
    #[clap(help = "HTTP method (GET/POST/PUT/DELETE etc.)")]
    method: String,
    #[clap(
        value_parser = OsStringValueParser::new().map(|s| Url::parse(s.to_str().unwrap())), 
        help = "Absolute or relative URL (profile must be configured for relative)"
    )]
    url: Url,
    #[clap(help = "body text to send with the request")]
    body: Option<String>,
    #[clap(short = 'p', long, default_value = "default", help = "profile name")]
    profile: String,
    #[clap(short = 'u', long, help = "username for basic authentication")]
    user: Option<String>,
    #[clap(short = 'w', long, help = "password for basic authentication")]
    password: Option<String>,
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
        name = "KEY:VALUE",
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

    #[clap(
        short = 'x',
        long,
        help = "HTTP proxy URL in <scheme>://<host>:<port> format",
        value_parser = OsStringValueParser::new().map(|s| Endpoint::parse(s.to_str().unwrap()))
    )]
    proxy: Option<Endpoint>
}

#[derive(Debug, Clone)]
pub struct CommandLineArgs {
    method: String,
    url: Url,
    body: Option<String>,
    profile: String,
    user: Option<String>,
    password: Option<String>,
    ca_cert: Option<String>,
    insecure: Option<bool>,
    headers: HashMap<String, String>,
    verbose: bool,
    proxy: Option<Endpoint>,
}

fn vec_to_hashmap(vec: Vec<String>) -> HashMap<String, String> {
    vec.into_iter()
        .map(|s| {
            let parts: Vec<&str> = s.splitn(2, ':').collect();
            if parts.len() != 2 {
                panic!("Invalid header format: {}", s);
            }
            (
                parts[0].trim().to_string().to_lowercase(),
                parts[1].trim().to_string(),
            )
        })
        .collect::<HashMap<String, String>>()
}

impl CommandLineArgs {
    pub fn parse() -> Self {
        let args = ClapArgs::parse();
        Self {
            verbose: args.verbose,
            method: args.method,
            url: args.url,
            body: args.body,
            profile: args.profile,
            user: args.user,
            password: args.password,
            ca_cert: args.ca_cert,
            insecure: Some(args.insecure),
            headers: vec_to_hashmap(args.headers),
            proxy: args.proxy,
        }
    }

    #[allow(dead_code)] // for testing
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
            insecure: Some(args.insecure),
            headers: vec_to_hashmap(args.headers),
            verbose: args.verbose,
            proxy: args.proxy,
        }
    }

    pub fn merge_req(&mut self, other: &dyn HttpRequestArgs) -> &mut Self {
        if other.url_path().is_some() {
            self.url.set_path(&other.url_path().unwrap());
        }

        if other.method().is_some() {
            self.method = other.method().unwrap().to_string();
        }

        if other.body().is_some() {
            // TODO: Reuse current allocated object
            self.body = Some(other.body().unwrap().to_string());
        }

        for (key, value) in other.headers() {
            let key = key.to_lowercase();
            self.headers.insert(key, value.clone());
        }

        self
    }

    pub fn profile(&self) -> &String {
        &self.profile
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }
}

impl HttpRequestArgs for CommandLineArgs {
    fn method(&self) -> Option<&String> {
        Some(&self.method)
    }

    fn url_path(&self) -> Option<&UrlPath> {
        self.url.to_url_path()
    }

    fn body(&self) -> Option<&String> {
        self.body.as_ref()
    }

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

impl HttpConnectionProfile for CommandLineArgs {
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

    fn server(&self) -> Option<&Endpoint> {
        self.url.to_endpoint()
    }

    fn proxy(&self) -> Option<&Endpoint> {
        self.proxy.as_ref()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const TEST_METHOD: &str = "GET";
    const TEST_HOST: &str = "example.com";
    const TEST_PORT: u16 = 8001;
    const TEST_SCHEME: &str = "https";
    const TEST_URL_PATH: &str = "/path/to/resource";
    const TEST_QUERY: &str = "query=foo";
    const TEST_BODY: &str = "{ \"query\": { \"match_all\": {} } }";
    const TEST_PROFILE: &str = "default";
    const TEST_USER: &str = "user";
    const TEST_PASSWORD: &str = "password";
    const TEST_CA_CERT: &str = "/path/to/ca_cert.pem";
    const TEST_INSECURE: bool = true;
    const TEST_HEADER_CONTENT_TYPE: &str = "Content-Type: application/json";
    const TEST_HEADER_USER_AGENT: &str = "User-Agent: wiq/0.0.1-SNAPSHOT";

    // struct TestArgs {
    //     method: String,
    //     url: Url,
    //     body: String,
    //     headers: HashMap<String, String>,
    // }

    // impl TestArgs {
    //     fn new(method: &str, url: &Url, body: &str, headers: HashMap<String, String>) -> Self {
    //         TestArgs {
    //             method: method.to_string(),
    //             url: url.clone(),
    //             body: body.to_string(),
    //             headers: headers.clone(),
    //         }
    //     }
    // }
    // impl HttpRequestArgs for TestArgs {
    //     fn method(&self) -> Option<&String> {
    //         Some(&self.method)
    //     }

    //     fn url_path(&self) -> Option<&UrlPath> {
    //         self.url.to_url_path()
    //     }

    //     fn body(&self) -> Option<&String> {
    //         Some(&self.body)
    //     }

    //     fn headers(&self) -> &HashMap<String, String> {
    //         &self.headers
    //     }
    // }

    #[test]
    fn test_cli() {
        use clap::CommandFactory;
        ClapArgs::command().debug_assert()
    }

    #[test]
    fn cmd_args_parse_should_decompose_values_properly() {
        let url = format!(
            "{}://{}:{}{}?{}",
            TEST_SCHEME, TEST_HOST, TEST_PORT, TEST_URL_PATH, TEST_QUERY
        );
        let params = vec![
            "http",
            TEST_METHOD,
            url.as_str(),
            TEST_BODY,
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
        let req: &dyn HttpRequestArgs = &args;
        let req_url_path = req.url_path().unwrap();

        assert_eq!(req.method().unwrap(), TEST_METHOD);
        assert_eq!(req_url_path.path(), &TEST_URL_PATH.to_string());
        assert_eq!(req_url_path.query(), Some(&TEST_QUERY.to_string()));
        assert_eq!(req.body().unwrap(), &TEST_BODY.to_string());

        assert_eq!(args.profile, TEST_PROFILE);
        let profile: &dyn HttpConnectionProfile = &args;
        assert_eq!(profile.user().unwrap(), &TEST_USER.to_string());
        assert_eq!(profile.password().unwrap(), &TEST_PASSWORD.to_string());
        assert_eq!(profile.ca_cert().unwrap(), &TEST_CA_CERT.to_string());
        assert_eq!(profile.insecure(), Some(TEST_INSECURE));
        assert_eq!(profile.headers().len(), 2);
        assert_eq!(
            profile.headers().get("content-type").unwrap(),
            TEST_HEADER_CONTENT_TYPE.split(":").nth(1).unwrap().trim()
        );
        assert_eq!(
            profile.headers().get("user-agent").unwrap(),
            TEST_HEADER_USER_AGENT.split(":").nth(1).unwrap().trim()
        );
    }

    // #[test]
    // fn cmd_arg_merge_req_should_merge_method_host_body_properly() {
    //     let params = vec![
    //         "http",
    //         TEST_METHOD,
    //         TEST_URL,
    //         TEST_BODY,
    //         "-p",
    //         TEST_PROFILE,
    //         "-u",
    //         TEST_USER,
    //         "-w",
    //         TEST_PASSWORD,
    //         "-r",
    //         TEST_CA_CERT,
    //         "-k",
    //         "-H",
    //         TEST_HEADER_CONTENT_TYPE,
    //     ];

    //     let cmd_args = CommandLineArgs::parse_from(params.iter());
    //     let req: Box<dyn HttpRequestArgs> = Box::new(TestArgs::new(
    //         "POST",
    //         &Url::parse(TEST_URL_2),
    //         "",
    //         vec_to_hashmap(vec![
    //             TEST_HEADER_USER_AGENT.to_string(),
    //             TEST_HEADER_CONTENT_TYPE.to_string(),
    //         ]),
    //     ));
    //     let merged_cmd = cmd_args.merge_req(req.as_ref());
    //     let merged_req: &dyn HttpRequestArgs = &merged_cmd;
    //     let merged_url: &Endpoint = merged_req.url_path().unwrap();
    //     let merged_headers = merged_req.headers();

    //     assert_eq!(merged_req.method(), Some(&"POST".to_string()));
    //     assert_eq!(merged_url.host(), Some(&"example.com".to_string()));
    //     assert_eq!(merged_req.body(), Some(&"".to_string()));
    //     assert_eq!(merged_url.port(), None);
    //     assert_eq!(merged_url.path(), Some(&"/path/to/resource".to_string()));
    //     assert_eq!(merged_url.query(), Some(&"query=foo".to_string()));
    //     assert_eq!(merged_url.scheme(), Some(&"https".to_string()));
    //     assert_eq!(
    //         merged_url.to_string(),
    //         "https://example.com/path/to/resource?query=foo"
    //     );
    //     assert_eq!(merged_headers.len(), 2);
    // }
}
