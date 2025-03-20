mod args;
mod http;

use args::{
    cmd::CommandLineArgs,
    ini::{DEFAULT_INI_FILE_PATH, DEFAULT_INI_SECTION, IniFile},
};
use http::{RequestArgs, send_request};
use reqwest::StatusCode;
use std::io::{Read, stdin};
use tokio;

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

fn ask_string(msg: &str) -> Result<String, Box<dyn std::error::Error>> {
    if msg.len() > 0 {
        eprint!("{}", msg);
    }

    let mut buffer = String::new();
    stdin().read_line(&mut buffer)?;
    buffer.pop(); // remove last \n
    Ok(buffer)
}

fn create_default_profile() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Looks like you haven't configured the default profile yet. Let's create it now.");

    let host = ask_string("host: ")?;
    let user = ask_string("user: ")?;
    let password = if user.len() > 0 {
        ask_string("password: ")?
    } else {
        "".to_string()
    };

    println!("{host:?} | {user:?} | {password:?}");
    Ok(())
}

fn get_request_args(cmd_args: &CommandLineArgs) -> Result<RequestArgs, Box<dyn std::error::Error>> {
    let cmd_url = cmd_args.url();
    let cmd_profile = cmd_args.profile();

    let ini_profile = IniFile::load_profile(DEFAULT_INI_FILE_PATH, cmd_profile.as_str())?;
    let ini_host = ini_profile.as_ref().and_then(|i| i.host());
    let ini_user = ini_profile.as_ref().and_then(|i| i.user());
    let ini_password = ini_profile.as_ref().and_then(|i| i.password());
    let ini_api_key = ini_profile.as_ref().and_then(|i| i.api_key());
    let ini_insecure = ini_profile.as_ref().and_then(|i| Some(i.insecure()));
    let ini_content_type = ini_profile.as_ref().and_then(|i| i.content_type());
    let ini_ca_cert = ini_profile.as_ref().and_then(|i| i.ca_cert());

    let url: String = match ini_profile {
        Some(_) => {
            if cmd_url.starts_with("http") {
                cmd_url
            } else {
                let mut host = ini_host.unwrap_or("http://localhost".to_string());
                host = if host.ends_with("/") {
                    host.pop();
                    host
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
