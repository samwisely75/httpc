use std::{collections::HashMap, ffi::OsString};

use clap::builder::{OsStringValueParser, TypedValueParser};
pub use clap::Parser;

use crate::http::{HttpConnectionProfile, HttpRequestArgs};
use crate::url::{Endpoint, Url, UrlPath};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ClapArgs {
    /// Method
    /// Optional. A HTTP method text that must be one of the ones defined in RFC 7231.
    /// All letter will be transformed to upper case. If omitted, interactive mode is enabled.
    #[clap(
        help = "HTTP method (GET/POST/PUT/DELETE/HEAD etc.)",
        value_parser = OsStringValueParser::new().map(|s| s.to_str().unwrap().to_uppercase() as String),
    )]
    method: Option<String>,

    /// URL
    /// Optional. String will be translated into Url object. If omitted, interactive mode is enabled.
    #[clap(
        value_parser = OsStringValueParser::new().map(|s| Url::parse(s.to_str().unwrap())),
        help = "Absolute or relative URL (profile must be configured for relative)"
    )]
    url: Option<Url>,

    /// Body
    /// Optional. Body text to send with the request.
    #[clap(help = "body text to send with the request")]
    body: Option<String>,

    /// Profile name
    /// Required. Profile name to use for the request. Default is 'default'.
    /// If the profile is not configured, the request will fail.
    #[clap(short = 'p', long, default_value = "default", help = "profile name")]
    profile: String,

    /// User
    /// Optional. Username for basic authentication.
    #[clap(short = 'u', long, help = "username for basic authentication")]
    user: Option<String>,

    /// Password
    /// Optional. Plaintext password for basic authentication.
    #[clap(short = 'w', long, help = "password for basic authentication")]
    password: Option<String>,

    /// CA certificate
    /// Optional. Path to the CA certificate PEM format file.
    #[clap(short = 'r', long, help = "CA certificate PEM file path")]
    ca_cert: Option<String>,

    /// Insecure
    /// Optional. Allow insecure server connections when using SSL.
    /// Same with the --insecure (-k) in curl.
    #[clap(
        short = 'k',
        long,
        help = "Allow insecure server connections when using SSL"
    )]
    insecure: bool,

    /// Headers
    /// Optional. HTTP headers to send with the request.
    /// Format: KEY:VALUE. Multiple headers can be specified.
    #[clap(
        short = 'H',
        long = "header",
        name = "KEY:VALUE",
        help = "HTTP header to send with the request. Multiple values can be specified by repeating the flag."
    )]
    headers: Vec<String>,

    /// Verbose mode
    /// Optional. Print verbose messages.
    #[clap(
        short = 'v',
        long,
        help = "Print verbose message",
        default_value = "false"
    )]
    verbose: bool,

    /// Proxy
    /// Optional. HTTP proxy URL in <scheme>://<host>:<port> format.
    /// If not specified, the request will be sent directly to the server.
    /// Same with the --proxy (-x) in curl.
    /// The string will be translated into Endpoint object.
    #[clap(
        short = 'x',
        long,
        help = "HTTP proxy URL in <scheme>://<host>:<port> format",
        value_parser = OsStringValueParser::new().try_map(|s| Endpoint::parse(s.to_str().unwrap()))
    )]
    proxy: Option<Endpoint>,
}

#[derive(Debug, Clone)]
pub struct CommandLineArgs {
    method: Option<String>,
    url: Option<Url>,
    body: Option<String>,
    #[allow(dead_code)] // Used by profile() method
    profile: String,
    user: Option<String>,
    password: Option<String>,
    ca_cert: Option<String>,
    insecure: Option<bool>,
    headers: HashMap<String, String>,
    #[allow(dead_code)] // Used in future features
    verbose: bool,
    proxy: Option<Endpoint>,
}

#[allow(dead_code)]
fn vec_to_hashmap(vec: Vec<String>) -> HashMap<String, String> {
    vec.into_iter()
        .map(|s| {
            let parts: Vec<&str> = s.splitn(2, ':').collect();
            if parts.len() != 2 {
                panic!("Invalid header format: {s}");
            }
            (
                parts[0].trim().to_string().to_lowercase(),
                parts[1].trim().to_string(),
            )
        })
        .collect::<HashMap<String, String>>()
}

impl CommandLineArgs {
    #[allow(dead_code)]
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
            insecure: if args.insecure { Some(true) } else { None },
            headers: vec_to_hashmap(args.headers),
            proxy: args.proxy,
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
            insecure: if args.insecure { Some(true) } else { None },
            headers: vec_to_hashmap(args.headers),
            verbose: args.verbose,
            proxy: args.proxy,
        }
    }

    #[allow(dead_code)]
    pub fn merge_req(&mut self, other: &dyn HttpRequestArgs) -> &mut Self {
        if let Some(url) = &mut self.url {
            if other.url_path().is_some() {
                url.set_path(other.url_path().unwrap());
            }
        }

        if other.method().is_some() {
            self.method = Some(other.method().unwrap().to_string());
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

    #[allow(dead_code)]
    pub fn profile(&self) -> &String {
        &self.profile
    }

    #[allow(dead_code)]
    pub fn verbose(&self) -> bool {
        self.verbose
    }

    #[allow(dead_code)]
    pub fn is_interactive(&self) -> bool {
        self.method.is_none() && self.url.is_none()
    }
}

impl HttpRequestArgs for CommandLineArgs {
    fn method(&self) -> Option<&String> {
        self.method.as_ref()
    }

    fn url_path(&self) -> Option<&UrlPath> {
        self.url.as_ref().and_then(|url| url.to_url_path())
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
        self.url.as_ref().and_then(|url| url.to_endpoint())
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
        let url = format!("{TEST_SCHEME}://{TEST_HOST}:{TEST_PORT}{TEST_URL_PATH}?{TEST_QUERY}");
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

    #[test]
    fn test_merge_req_functionality() {
        let mut cmd_args = CommandLineArgs::parse_from([
            "http",
            "GET",
            "https://example.com/original",
            "original body",
            "-H",
            "Original-Header: original-value",
        ]);

        // Create a mock HttpRequestArgs to merge
        let mut headers = HashMap::new();
        headers.insert("new-header".to_string(), "new-value".to_string());
        headers.insert(
            "original-header".to_string(),
            "overridden-value".to_string(),
        );

        let stdin_args = MockStdinArgs {
            method: Some("POST".to_string()),
            url_path: Some(crate::url::UrlPath::new(
                "/new/path".to_string(),
                Some("query=test".to_string()),
            )),
            body: Some("new body".to_string()),
            headers,
        };

        cmd_args.merge_req(&stdin_args);

        // Check that values were merged correctly
        assert_eq!(cmd_args.method().unwrap(), "POST");
        assert_eq!(cmd_args.body().unwrap(), "new body");

        let request_headers: &dyn HttpRequestArgs = &cmd_args;
        assert_eq!(
            request_headers.headers().get("new-header").unwrap(),
            "new-value"
        );
        assert_eq!(
            request_headers.headers().get("original-header").unwrap(),
            "overridden-value"
        );

        assert_eq!(
            cmd_args.url.as_ref().unwrap().path(),
            Some(&"/new/path".to_string())
        );
        assert_eq!(
            cmd_args.url.as_ref().unwrap().query(),
            Some(&"query=test".to_string())
        );
    }

    #[test]
    fn test_merge_req_partial_override() {
        let mut cmd_args = CommandLineArgs::parse_from([
            "http",
            "GET",
            "https://example.com/path",
            "original body",
        ]);

        // Create a mock HttpRequestArgs with only some fields
        let stdin_args = MockStdinArgs {
            method: None,                       // Don't override method
            url_path: None,                     // Don't override URL path
            body: Some("new body".to_string()), // Override body
            headers: HashMap::new(),            // No headers
        };

        cmd_args.merge_req(&stdin_args);

        // Check that only specified values were overridden
        assert_eq!(cmd_args.method().unwrap(), "GET"); // Original method preserved
        assert_eq!(cmd_args.body().unwrap(), "new body"); // Body overridden
        assert_eq!(
            cmd_args.url.as_ref().unwrap().path(),
            Some(&"/path".to_string())
        ); // Original path preserved
    }

    #[test]
    fn test_vec_to_hashmap_valid_headers() {
        let headers = vec![
            "Content-Type: application/json".to_string(),
            "Authorization: Bearer token123".to_string(),
            "Custom-Header:   custom-value   ".to_string(), // Test trimming
        ];

        let result = vec_to_hashmap(headers);

        assert_eq!(result.len(), 3);
        assert_eq!(result.get("content-type").unwrap(), "application/json");
        assert_eq!(result.get("authorization").unwrap(), "Bearer token123");
        assert_eq!(result.get("custom-header").unwrap(), "custom-value");
    }

    #[test]
    #[should_panic(expected = "Invalid header format")]
    fn test_vec_to_hashmap_invalid_header_no_colon() {
        let headers = vec!["InvalidHeader".to_string()];
        vec_to_hashmap(headers);
    }

    #[test]
    #[should_panic(expected = "Invalid header format")]
    fn test_vec_to_hashmap_invalid_header_empty() {
        let headers = vec!["".to_string()];
        vec_to_hashmap(headers);
    }

    #[test]
    fn test_vec_to_hashmap_header_with_multiple_colons() {
        let headers = vec!["Content-Type: application/json; charset=utf-8".to_string()];
        let result = vec_to_hashmap(headers);

        assert_eq!(result.len(), 1);
        assert_eq!(
            result.get("content-type").unwrap(),
            "application/json; charset=utf-8"
        );
    }

    #[test]
    fn test_profile_and_verbose_getters() {
        let args = CommandLineArgs::parse_from([
            "http",
            "GET",
            "https://example.com",
            "-p",
            "custom-profile",
            "-v",
        ]);

        assert_eq!(args.profile(), "custom-profile");
        assert!(args.verbose());
    }

    #[test]
    fn test_default_profile_and_verbose() {
        let args = CommandLineArgs::parse_from(["http", "GET", "https://example.com"]);

        assert_eq!(args.profile(), "default");
        assert!(!args.verbose());
    }

    #[test]
    fn test_http_connection_profile_implementation() {
        let args = CommandLineArgs::parse_from([
            "http",
            "GET",
            "https://user:pass@example.com:8080/path",
            "-u",
            "testuser",
            "-w",
            "testpass",
            "-k",
            "-r",
            "/path/to/cert.pem",
            "-H",
            "X-Custom: value",
            "-x",
            "http://proxy.example.com:3128",
        ]);

        let profile: &dyn HttpConnectionProfile = &args;

        // Test server endpoint
        assert!(profile.server().is_some());
        let server = profile.server().unwrap();
        assert_eq!(server.host(), "example.com");
        assert_eq!(server.port(), Some(8080));
        assert_eq!(server.scheme(), Some(&"https".to_string()));

        // Test authentication
        assert_eq!(profile.user().unwrap(), "testuser");
        assert_eq!(profile.password().unwrap(), "testpass");

        // Test SSL settings
        assert_eq!(profile.insecure(), Some(true));
        assert_eq!(profile.ca_cert().unwrap(), "/path/to/cert.pem");

        // Test headers
        assert_eq!(profile.headers().get("x-custom").unwrap(), "value");

        // Test proxy
        assert!(profile.proxy().is_some());
        let proxy = profile.proxy().unwrap();
        assert_eq!(proxy.host(), "proxy.example.com");
        assert_eq!(proxy.port(), Some(3128));
    }

    #[test]
    fn test_http_request_args_implementation() {
        let args = CommandLineArgs::parse_from([
            "http",
            "POST",
            "https://example.com/api/test?param=value",
            "request body content",
            "-H",
            "Content-Type: application/json",
        ]);

        let request: &dyn HttpRequestArgs = &args;

        assert_eq!(request.method().unwrap(), "POST");
        assert_eq!(request.body().unwrap(), "request body content");

        let url_path = request.url_path().unwrap();
        assert_eq!(url_path.path(), "/api/test");
        assert_eq!(url_path.query(), Some(&"param=value".to_string()));

        assert_eq!(
            request.headers().get("content-type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_method_case_normalization() {
        let args = CommandLineArgs::parse_from([
            "http",
            "post", // lowercase
            "https://example.com",
        ]);

        assert_eq!(args.method().unwrap(), "POST"); // Should be uppercase
    }

    #[test]
    fn test_insecure_flag_handling() {
        // Test with insecure flag
        let args_insecure =
            CommandLineArgs::parse_from(["http", "GET", "https://example.com", "-k"]);
        assert_eq!(args_insecure.insecure(), Some(true));

        // Test without insecure flag
        let args_secure = CommandLineArgs::parse_from(["http", "GET", "https://example.com"]);
        assert_eq!(args_secure.insecure(), None);
    }

    // Helper struct for testing merge_req
    #[derive(Debug)]
    struct MockStdinArgs {
        method: Option<String>,
        url_path: Option<crate::url::UrlPath>,
        body: Option<String>,
        headers: HashMap<String, String>,
    }

    impl HttpRequestArgs for MockStdinArgs {
        fn method(&self) -> Option<&String> {
            self.method.as_ref()
        }

        fn url_path(&self) -> Option<&crate::url::UrlPath> {
            self.url_path.as_ref()
        }

        fn body(&self) -> Option<&String> {
            self.body.as_ref()
        }

        fn headers(&self) -> &HashMap<String, String> {
            &self.headers
        }
    }
}
