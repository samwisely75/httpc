mod args;
mod http;

use args::{cmd::CommandLineArgs, ini::IniSectionArgs};
use http::{RequestArgs, send_request};
use reqwest::StatusCode;
use std::io::{Read, stdin};
use tokio;

const DEFAULT_INI_FILE_PATH: &str = "~/.http";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd_args = CommandLineArgs::get();
    let req_args = get_request_args(&cmd_args)?;

    if cmd_args.verbose() {
        eprintln!("> {} {}", req_args.method, req_args.url);
    }

    let res = send_request(req_args).await?;

    if cmd_args.verbose() {
        eprintln!("> status: {}", res.status());
        for (name, value) in res.headers().iter() {
            eprintln!("> {}: {}", name.to_string(), value.to_str()?);
        }
    }

    if res.status() == StatusCode::OK {
        println!("{}", res.body());
    } else {
        eprintln!("{}: {}", res.status(), res.body());
    }

    Ok(())
}

fn get_request_args(cmd_args: &CommandLineArgs) -> Result<RequestArgs, Box<dyn std::error::Error>> {
    let cmd_url = cmd_args.url();
    let ini_args = IniSectionArgs::from_file(DEFAULT_INI_FILE_PATH, cmd_args.profile().as_str());
    let ini_host = ini_args.as_ref().and_then(|i| i.host());
    let ini_user = ini_args.as_ref().and_then(|i| i.user());
    let ini_password = ini_args.as_ref().and_then(|i| i.password());
    let ini_api_key = ini_args.as_ref().and_then(|i| i.api_key());
    let ini_insecure = ini_args.as_ref().and_then(|i| Some(i.insecure()));
    let ini_content_type = ini_args.as_ref().and_then(|i| i.content_type());
    let ini_ca_cert = ini_args.as_ref().and_then(|i| i.ca_cert());

    let url: String = match ini_args {
        Some(_) => {
            if cmd_url.starts_with("http") {
                cmd_url
            } else {
                let mut host = ini_host.unwrap_or("http://localhost".to_string());
                host = if host.ends_with("/") {
                    host[..(host.len() - 1)].to_string()
                } else {
                    host
                };
                let path = if cmd_url.starts_with("/") {
                    cmd_url[1..].to_string()
                } else {
                    cmd_url
                };
                format!("{}/{}", host, path)
            }
        }
        None => cmd_url,
    };

    let user = cmd_args.user().and_then(|u| Some(u)).or(ini_user);
    let password = cmd_args.password().and_then(|p| Some(p)).or(ini_password);
    let api_key = cmd_args.api_key().and_then(|a| Some(a)).or(ini_api_key);
    let insecure = if cmd_args.insecure() {
        true
    } else {
        ini_insecure.unwrap_or(false)
    };
    let ca_cert = cmd_args.ca_cert().and_then(|c| Some(c)).or(ini_ca_cert);
    let content_type = cmd_args
        .content_type()
        .and_then(|c| Some(c))
        .or(ini_content_type);

    let body_text = if cmd_args.stdin() {
        dbg!("Reading from stdin ...");
        let mut buffer = String::new();
        stdin().read_to_string(&mut buffer)?;
        Some(buffer)
    } else {
        cmd_args.text()
    };

    Ok(RequestArgs {
        method: cmd_args.method(),
        url: url,
        body: body_text,
        user: user,
        password: password,
        insecure: insecure,
        ca_certs: ca_cert,
        api_key: api_key,
        content_type: content_type,
    })
}
