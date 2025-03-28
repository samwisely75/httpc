mod http;
mod cmd;
mod encoder;
mod ini;
mod utils;
mod stdio;

use http::{HttpClient, HttpRequest, RequestArgs};
use cmd::CommandLineArgs;
use ini::{DEFAULT_INI_FILE_PATH, IniFileArgs};
use stdio::StdinArgs;
use reqwest::StatusCode;
use utils::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let stdin_args = StdinArgs::new(&mut std::io::stdin())?;
    let cmd_args = CommandLineArgs::parse();
    let url_starts_with_http = cmd_args.url().map(|url| url.scheme().unwrap_or(&"".to_string()).starts_with("http")).unwrap_or(false);
    let ini_args = if url_starts_with_http { None } else { IniFileArgs::load(DEFAULT_INI_FILE_PATH, &cmd_args.profile())? };

    let req_args: HttpRequest = if let Some(ini) = ini_args {
        // stdin > command line > ini
        HttpRequest::from(&ini).merge(&cmd_args).merge(&stdin_args).clone()
    } else {
        // stdin > command line
        HttpRequest::from(&cmd_args).merge(&stdin_args).clone()
    };

    dbg!(&req_args);

    if cmd_args.verbose() {
        eprintln!("> request:");
        eprintln!(">   method: {}", req_args.method().unwrap());
        eprintln!(">   url: {}", req_args.url().unwrap());
        eprintln!(">   headers:");
        req_args.headers().iter().for_each(|(name, value)| {
            eprintln!(">    {name}: {value}");
        });
    }

    let res = HttpClient::request(&req_args).await?;

    if cmd_args.verbose() {
        eprintln!("> response:");
        eprintln!(">   status: {}", res.status());
        eprintln!(">   headers:");
        res.headers().iter().for_each(|(name, value)| {
            eprintln!(">     {}: {}", name, value.to_str().unwrap());
        });
    }

    if res.status() == StatusCode::OK {
        println!("{}", res.body());
    } else {
        eprintln!("{}: {}", res.status(), res.body());
    }

    Ok(())
}
