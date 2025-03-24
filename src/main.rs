mod args;
mod http;
mod profile;
mod utils;

use args::CommandLineArgs;
use http::{RequestArgs, send_request};
use profile::{DEFAULT_INI_FILE_PATH, DEFAULT_INI_SECTION, IniFile};
use reqwest::StatusCode;
use std::collections::HashMap;
use utils::{Result, read_stdin};

#[tokio::main]
async fn main() -> Result<()> {
    let cmd_args = CommandLineArgs::get();
    let req_args = get_request_args(&cmd_args)?;

    if cmd_args.verbose() {
        req_args.headers.iter().for_each(|(name, value)| {
            eprintln!("> {}: {}", name.to_string(), value.to_string());
        });
        eprintln!("> {} {}", req_args.method(), req_args.url());
    }

    let res = send_request(&req_args).await?;

    if cmd_args.verbose() {
        eprintln!("> status: {}", res.status());
        res.headers().iter().for_each(|(name, value)| {
            eprintln!("> {}: {}", name.to_string(), value.to_str().unwrap());
        });
    }

    if res.status() == StatusCode::OK {
        println!("{}", res.body());
    } else {
        eprintln!("{}: {}", res.status(), res.body());
    }

    Ok(())
}

fn get_request_args(cmd_args: &CommandLineArgs) -> Result<RequestArgs> {
    let cmd_url = cmd_args.url();
    let profile_name = cmd_args.profile();

    let no_valid_url = profile_name == DEFAULT_INI_SECTION
        && !IniFile::profile_exists(DEFAULT_INI_FILE_PATH, DEFAULT_INI_SECTION)
        && !cmd_args.url().starts_with("http");

    let profile = if no_valid_url {
        eprintln!("Let's create a default profile now.");
        let p = IniFile::ask_profile()?;
        IniFile::add_profile(DEFAULT_INI_FILE_PATH, &profile_name, &p)?;
        Some(p)
    } else {
        IniFile::load_profile(DEFAULT_INI_FILE_PATH, profile_name.as_str())?
    };

    let prof_ref = profile.as_ref();

    let url = if cmd_url.starts_with("http") {
        cmd_url
    } else {
        format!(
            "{}{}",
            prof_ref.and_then(|i| i.host()).unwrap_or("".to_string()),
            cmd_url
        )
    };
    let user = cmd_args.user().or(prof_ref.and_then(|p| p.user()));
    let password = cmd_args.password().or(prof_ref.and_then(|p| p.password()));
    let api_key = cmd_args.api_key().or(prof_ref.and_then(|p| p.api_key()));
    let ca_certs = cmd_args.ca_cert().or(prof_ref.and_then(|p| p.ca_cert()));
    let insecure =
        cmd_args.insecure() || prof_ref.and_then(|p| Some(p.insecure())).unwrap_or(false);
    let headers = profile
        .and_then(|p| Some(p.headers))
        .unwrap_or(HashMap::new());
    let body_text = read_stdin()?.or(cmd_args.text().map(|s| s.to_string()));
    let req_args = RequestArgs::new(
        cmd_args.method(),
        url,
        body_text,
        user,
        password,
        api_key,
        insecure,
        ca_certs,
        headers,
    );
    Ok(req_args)
}
