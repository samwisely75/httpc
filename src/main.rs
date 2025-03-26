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
    let profile = get_or_create_profile(cmd_args)?;
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
    let headers = merge_headers(
        prof_ref.and_then(|p| Some(p.headers.clone())),
        cmd_args.headers(),
    );
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

fn get_or_create_profile(cmd_args: &CommandLineArgs) -> Result<Option<profile::Profile>> {
    let profile_name = cmd_args.profile();
    let cmd_url = cmd_args.url();
    let no_valid_url = profile_name == DEFAULT_INI_SECTION
        && !IniFile::profile_exists(DEFAULT_INI_FILE_PATH, DEFAULT_INI_SECTION)
        && !cmd_url.starts_with("http");

    let profile = if no_valid_url {
        eprintln!("Let's create a default profile now.");
        let p = IniFile::ask_profile()?;
        IniFile::add_profile(DEFAULT_INI_FILE_PATH, &profile_name, &p)?;
        Some(p)
    } else {
        IniFile::load_profile(DEFAULT_INI_FILE_PATH, profile_name.as_str())?
    };
    Ok(profile)
}

fn merge_headers(
    prof_headers: Option<HashMap<String, String>>,
    cmd_headers: Vec<String>,
) -> HashMap<String, String> {
    let cmd_headers_trans = cmd_headers
        .iter()
        .map(|h| {
            let mut parts = h.split(':');
            (
                parts.next().unwrap().trim().to_string(),
                parts.next().unwrap().trim().to_string(),
            )
        })
        .collect::<HashMap<String, String>>();
    let headers = prof_headers
        .unwrap_or(HashMap::new())
        .into_iter()
        .chain(cmd_headers_trans)
        .collect();
    dbg!(&headers);
    headers
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_merge_headers() {
        let prof_headers = Some(
            vec![("Accept", "*/*"), ("Content-Type", "application/json")]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        let cmd_headers = vec!["Authorization: Bearer 1234", "Content-Type: text/plain"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let headers = merge_headers(prof_headers, cmd_headers);
        assert_eq!(headers.len(), 3);
        assert_eq!(headers.get("Accept").unwrap(), "*/*");
        assert_eq!(headers.get("Content-Type").unwrap(), "text/plain");
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer 1234");
    }
}
