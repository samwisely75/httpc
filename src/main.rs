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
    let cmd_args = CommandLineArgs::parse();
    let ini_args = IniFileArgs::load(DEFAULT_INI_FILE_PATH, &cmd_args.profile())?;
    let stdin_args = StdinArgs::new(&mut std::io::stdin())?;

    let req_args: HttpRequest = if let Some(ini) = ini_args {
        // stdin > command line > ini
        HttpRequest::from(&ini).merge(&cmd_args).merge(&stdin_args).clone()
    } else {
        // stdin > command line
        HttpRequest::from(&cmd_args).merge(&stdin_args).clone()
    };

    dbg!(&req_args);

    if cmd_args.verbose() {
        req_args.headers().iter().for_each(|(name, value)| {
            eprintln!("> {name}: {value}");
        });
        eprintln!(
            "> {} {}",
            req_args.method().unwrap(),
            req_args.url().unwrap()
        );
    }

    let res = HttpClient::request(&req_args).await?;

    if cmd_args.verbose() {
        eprintln!("> status: {}", res.status());
        res.headers().iter().for_each(|(name, value)| {
            eprintln!("> {}: {}", name, value.to_str().unwrap());
        });
    }

    if res.status() == StatusCode::OK {
        println!("{}", res.body());
    } else {
        eprintln!("{}: {}", res.status(), res.body());
    }

    Ok(())
}
