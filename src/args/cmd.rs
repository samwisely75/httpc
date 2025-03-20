pub use clap::Parser;

#[derive(Parser, Debug)]
#[command()]
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
    #[clap(short = 'i', long, help = "API key for authentication")]
    api_key: Option<String>,
    #[clap(short = 'r', long, help = "CA certificate PEM file path")]
    ca_cert: Option<String>,
    #[clap(
        short = 'k',
        long,
        help = "Allow insecure server connections when using SSL"
    )]
    insecure: bool,
    #[clap(short = 'c', long, help = "Content-Type header value")]
    content_type: Option<String>,
    #[clap(
        short = 'v',
        long,
        help = "Print verbose message",
        default_value = "false"
    )]
    verbose: bool,
    #[clap(short = 's', long, help = "Read from stdin")]
    stdin: bool,
}

impl CommandLineArgs {
    pub fn get() -> Self {
        CommandLineArgs::parse()
    }

    pub fn method(&self) -> String {
        self.method.clone()
    }

    pub fn url(&self) -> String {
        self.url.clone()
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

    pub fn content_type(&self) -> Option<String> {
        self.content_type.clone()
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn stdin(&self) -> bool {
        self.stdin
    }
}
