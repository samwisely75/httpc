mod cmd;
mod decoder;
mod http;
mod ini;
mod repl;
mod stdio;
mod url;
mod utils;

use cmd::CommandLineArgs;
use http::{HttpClient, HttpConnectionProfile, HttpRequestArgs, HttpResponse};
use ini::{get_blank_profile, IniProfileStore, DEFAULT_INI_FILE_PATH};
use repl::VimRepl;
use reqwest::StatusCode;
// use stdio::StdinArgs;  // Temporarily disabled for REPL
use tracing_subscriber::{fmt::time::ChronoLocal, EnvFilter};
use utils::Result;

#[tracing::instrument]
#[tokio::main]
async fn main() -> Result<()> {
    init_tracing_subscriber();

    // Load command line arguments
    let cmd_args = CommandLineArgs::parse();

    // Check if we should enter interactive mode
    if cmd_args.is_interactive() {
        return run_interactive_mode(&cmd_args).await;
    }

    // TODO: Re-enable stdin feature after REPL is fully implemented
    // For now, we skip stdin processing to avoid conflicts with REPL
    // Read user input from stdin and merge it into command line args.
    // This must happen before loading a profile which may use a
    // command prompt to complete the missing profile.
    // let mut stdin = std::io::stdin();
    // let stdin_args = StdinArgs::new(&mut stdin)?;
    // cmd_args.merge_req(&stdin_args);
    // tracing::debug!("stdin_args: {:?}", stdin_args);

    // Load profile from INI file by name specified in --profile argument
    // (default to "default")
    // If the profile is not found, then use a blank profile.
    let profile_name = cmd_args.profile();
    let ini_store = IniProfileStore::new(DEFAULT_INI_FILE_PATH);
    let mut profile = ini_store
        .get_profile(profile_name)?
        .unwrap_or(get_blank_profile());
    tracing::debug!("INI profile: {:?}", profile);

    // Merge the command line arguments (e.g. user, password, etc.)
    // to complete the connection profile. Note the server in profile
    // will be overwritten if a scheme and server is specified in
    // the command line URL
    profile.merge_profile(&cmd_args);
    tracing::debug!("Merged profile: {:?}", profile);

    // Show the connection profile and request details to stderr output
    // if verbose mode is enabled
    if cmd_args.verbose() {
        print_profile(&profile);
        print_request(&cmd_args);
    }

    // Send the request and print the response
    let res = HttpClient::new(&profile)?.request(&cmd_args).await?;
    tracing::debug!("Response: {:?}", res);

    // Print the response details to stderr if verbose mode is enabled
    if cmd_args.verbose() {
        print_response(&res);
    }

    print_result(&res);

    Ok(())
}

async fn run_interactive_mode(cmd_args: &CommandLineArgs) -> Result<()> {
    // Load profile from INI file by name specified in --profile argument
    // (default to "default")
    // If the profile is not found, then use a blank profile.
    let profile_name = cmd_args.profile();
    let ini_store = IniProfileStore::new(DEFAULT_INI_FILE_PATH);
    let mut profile = ini_store
        .get_profile(profile_name)?
        .unwrap_or(get_blank_profile());
    tracing::debug!("INI profile: {:?}", profile);

    // Merge the command line arguments (e.g. user, password, etc.)
    // to complete the connection profile.
    profile.merge_profile(cmd_args);
    tracing::debug!("Merged profile: {:?}", profile);

    // Create and run the VIM-like REPL
    let mut repl = VimRepl::new(profile, cmd_args.verbose())?;
    repl.run().await
}

fn print_result(res: &HttpResponse) {
    // Print the response body
    if res.status() == StatusCode::OK {
        if res.json().is_some() {
            println!(
                "{}",
                serde_json::to_string_pretty(res.json().as_ref().unwrap()).unwrap()
            );
        } else {
            println!("{}", res.body());
        }
    } else {
        eprintln!("{}: {}", res.status(), res.body());
    }
}

fn init_tracing_subscriber() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_env(format!(
                "{}_LOG_LEVEL",
                env!("CARGO_PKG_NAME").to_uppercase()
            ))
            .add_directive("reqwest=warn".parse().unwrap())
            .add_directive("hyper=warn".parse().unwrap())
            .add_directive("tokio=warn".parse().unwrap())
            .add_directive("tracing=warn".parse().unwrap())
            .add_directive("tracing_subscriber=warn".parse().unwrap())
            .add_directive("tower_http=warn".parse().unwrap())
            .add_directive("tower=warn".parse().unwrap())
            .add_directive("tokio_util=warn".parse().unwrap())
            .add_directive("tokio_rustls=warn".parse().unwrap())
            .add_directive("rustls=warn".parse().unwrap())
            .add_directive("rustls_pemfile=warn".parse().unwrap())
            .add_directive("native_tls=warn".parse().unwrap())
            .add_directive("tokio_util=warn".parse().unwrap())
            .add_directive("tokio_stream=warn".parse().unwrap())
            .add_directive("tokio_io=warn".parse().unwrap())
            .add_directive("tokio_timer=warn".parse().unwrap())
            .add_directive("tokio_sync=warn".parse().unwrap())
            .add_directive("tokio_task=warn".parse().unwrap())
            .add_directive("tokio_reactor=warn".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .with_timer(ChronoLocal::rfc_3339())
        .init();
}

#[tracing::instrument]
fn print_profile(profile: &impl HttpConnectionProfile) {
    if let Some(endpoint) = profile.server() {
        eprintln!("> connection:");
        eprintln!(">   host: {}", endpoint.host());
        eprintln!(
            ">   port: {}",
            endpoint
                .port()
                .map(|p| p.to_string())
                .unwrap_or("<none>".to_string())
        );
        eprintln!(">   scheme: {}", endpoint.scheme().unwrap());
        if endpoint.scheme().unwrap() == "https" {
            eprintln!(
                ">   ca-cert: {}",
                profile.ca_cert().unwrap_or(&"<none>".to_string())
            );
            eprintln!(
                ">   insecure: {}",
                profile
                    .insecure()
                    .map(|x| x.to_string())
                    .unwrap_or("<none>".to_string())
            );
        }
    } else {
        eprintln!("> connection: <none>");
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

    if profile.proxy().is_some() {
        eprintln!(">   proxy: {}", profile.proxy().unwrap());
    }
}

#[tracing::instrument]
fn print_request(req: &impl HttpRequestArgs) {
    let url = req
        .url_path()
        .map(|u| u.to_string())
        .unwrap_or("<none>".to_string());
    eprintln!("> request:");
    eprintln!(">   method: {}", req.method().unwrap());
    eprintln!(">   path: {url}");
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
