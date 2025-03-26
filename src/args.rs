pub use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CommandLineArgs {
    #[clap(help = "HTTP method (GET/POST/PUT/DELETE etc.)")]
    method: String,
    #[clap(help = "URL to send the request")]
    url: String,
    #[clap(help = "body text to send with the request")]
    text: Option<String>,
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
        short = 'v',
        long,
        help = "Print verbose message",
        default_value = "false"
    )]
    verbose: bool,
    #[clap(
        short = 'H',
        long = "header",
        help = "HTTP header to send with the request"
    )]
    headers: Vec<String>,
}

impl CommandLineArgs {
    pub fn get() -> Self {
        CommandLineArgs::parse()
    }

    pub fn method(&self) -> String {
        self.method.clone()
    }

    pub fn url(&self) -> String {
        let _url = &self.url;
        if _url.starts_with("http") || _url.starts_with("/") {
            _url.to_string()
        } else {
            format!("/{}", _url)
        }
    }

    pub fn text(&self) -> Option<String> {
        self.text.clone()
    }

    pub fn profile(&self) -> String {
        self.profile.clone()
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

    pub fn ca_cert(&self) -> Option<String> {
        self.ca_cert.clone()
    }

    pub fn insecure(&self) -> bool {
        self.insecure
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn headers(&self) -> Vec<String> {
        self.headers.clone()
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
        CommandLineArgs::command().debug_assert()
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
        assert_eq!(args.url, TEST_URL);
        assert_eq!(args.text, Some(TEST_TEXT.to_string()));
        assert_eq!(args.profile, TEST_PROFILE);
        assert_eq!(args.user, Some(TEST_USER.to_string()));
        assert_eq!(args.password, Some(TEST_PASSWORD.to_string()));
        assert_eq!(args.ca_cert, Some(TEST_CA_CERT.to_string()));
        assert_eq!(args.insecure, TEST_INSECURE);

        assert_eq!(args.headers.len(), 2);
        assert_eq!(args.headers[0], TEST_HEADER_CONTENT_TYPE);
        assert_eq!(args.headers[1], TEST_HEADER_USER_AGENT);
    }
}
