mod cmd;
mod encoder;
mod http;
mod ini;
mod stdio;
mod url;
mod utils;

use cmd::CommandLineArgs;
use http::{HttpClient, HttpConnectionProfile, HttpRequestArgs, HttpResponse};
use ini::{DEFAULT_INI_FILE_PATH, IniProfileStore, get_blank_profile};
use reqwest::StatusCode;
use stdio::StdinArgs;
use utils::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load command line arguments
    let mut cmd_args = CommandLineArgs::parse();

    // Read user input from stdin and merge it into command line args
    // This must happen before the profile loading which may use stdin
    // to complete the missing profile.
    let mut stdin = std::io::stdin();
    let stdin_args = StdinArgs::new(&mut stdin)?;
    cmd_args.merge_req(&stdin_args);

    // Load profile from INI file by name specified in --profile argument
    // (default to "default")
    // If the profile is not found, then use a blank profile.
    let profile_name = cmd_args.profile();
    let ini_store = IniProfileStore::new(DEFAULT_INI_FILE_PATH);
    let mut profile = ini_store
        .get_profile(&profile_name)?
        .unwrap_or(get_blank_profile());

    // Merge the command line arguments (e.g. user, password, etc.)
    // to complete the connection profile. Note the server in ptofile
    // will be overwritten if a scheme and server is specified in
    // the command line URL
    profile.merge_profile(&cmd_args);

    // Show the connection profile and request details to stderr output
    // if verbose mode is enabled
    if cmd_args.verbose() {
        print_profile(&profile);
        print_request(&cmd_args);
    }

    // Send the request and print the response
    let res = HttpClient::new(&profile).request(&cmd_args).await?;

    // Print the response details to stderr if verbose mode is enabled
    if cmd_args.verbose() {
        print_response(&res);
    }

    // Print the response body
    if res.status() == StatusCode::OK {
        println!("{}", res.body());
    } else {
        eprintln!("{}: {}", res.status(), res.body());
    }

    Ok(())
}

fn print_profile(profile: &impl HttpConnectionProfile) {
    let server = profile.server().unwrap();
    eprintln!("> connection:");
    eprintln!(">   host: {}", server.host());
    eprintln!(
        ">   port: {}",
        server
            .port()
            .map(|p| p.to_string())
            .unwrap_or("<none>".to_string())
    );
    eprintln!(">   scheme: {}", server.scheme().unwrap());
    if server.scheme().unwrap() == "https" {
        eprintln!(
            ">   ca-cert: {}",
            profile.ca_cert().unwrap_or(&"<none>".to_string())
        );
        eprintln!(">   verify-cert: {}", !profile.insecure().unwrap());
    } 

    if profile.user().is_some() {
        eprintln!(">   user: {}", profile.user().unwrap());
        eprintln!(
            ">   password: {}",
            profile.password().map(|_| "<provided>").unwrap_or("<none>")
        );
    }

    eprintln!(">   headers:");
    profile.headers().iter().for_each(|(name, value)| {
        eprintln!(">    {name}: {value}");
    });
}

fn print_request(req: &impl HttpRequestArgs) {
    eprintln!("> request:");
    eprintln!(">   method: {}", req.method().unwrap());
    eprintln!(">   url: {}", req.url_path().unwrap());
    eprintln!(">   path: {}", req.url_path().unwrap().path());
    eprintln!(
        ">   query: {}",
        req.url_path()
            .unwrap()
            .query()
            .unwrap_or(&"<none>".to_string())
    );
    eprintln!(
        ">   body: {}",
        req.body()
            .map(|b| if b.len() > 78 {
                format!("{}...", &b[0..75])
            } else {
                b.to_string()
            })
            .unwrap_or("<none>".to_string())
    );
}

fn print_response(res: &HttpResponse) {
    eprintln!("> response:");
    eprintln!(">   status: {}", res.status());
    eprintln!(">   headers:");
    res.headers().iter().for_each(|(name, value)| {
        eprintln!(">     {}: {}", name, value.to_str().unwrap());
    });
}
