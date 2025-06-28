use regex::Regex;

use crate::http::HttpRequestArgs;
use crate::url::UrlPath;
use crate::utils::Result;
use std::collections::HashMap;
use std::io::{Read, Stdin};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdinArgs {
    input: Option<String>,
    headers: HashMap<String, String>,
}

#[allow(dead_code)]
impl StdinArgs {
    pub fn new(i: &mut Stdin) -> Result<Self> {
        if atty::is(atty::Stream::Stdin) {
            return Ok(Self {
                input: None,
                headers: HashMap::new(),
            });
        }

        let mut input = String::new();
        i.read_to_string(&mut input)?;

        Ok(Self {
            input: Some(input),
            headers: HashMap::new(),
        })
    }
}

impl HttpRequestArgs for StdinArgs {
    fn method(&self) -> Option<&String> {
        None
    }

    fn url_path(&self) -> Option<&UrlPath> {
        None
    }

    fn body(&self) -> Option<&String> {
        self.input.as_ref()
    }

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

pub fn ask<T>(i: &Stdin, msg: &str, acceptable: &str) -> Result<T>
where
    T: std::str::FromStr,
{
    let re = Regex::new(acceptable)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid regex"))?;

    loop {
        if !msg.is_empty() {
            eprint!("{}", msg);
        }

        let mut buffer = String::new();
        i.read_line(&mut buffer)?;
        buffer.pop(); // remove last \n

        if re.is_match(&buffer) {
            if let Ok(value) = buffer.parse::<T>() {
                return Ok(value);
            }
        }
    }
}

pub fn ask_no_space_string(i: &Stdin, msg: &str) -> Result<String> {
    ask::<String>(i, msg, r"^[^\s\t]+$")
}

pub fn ask_binary(i: &Stdin, msg: &str) -> Result<bool> {
    let buffer = ask::<String>(i, msg, r"^[YyNn]$")?;
    Ok(buffer.to_lowercase() == "y")
}

pub fn ask_path(i: &Stdin, msg: &str) -> Result<String> {
    loop {
        let str = ask_no_space_string(i, msg)?;
        let expand_str = shellexpand::tilde(str.as_str()).to_string();
        let expand_path = Path::new(&expand_str);
        if expand_path.exists() {
            return Ok(expand_path.to_str().unwrap().to_string());
        } else {
            eprintln!("Path not found: {}", str);
        }
    }
}
