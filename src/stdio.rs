use crate::http::RequestArgs;
use crate::utils::Result;
use std::collections::HashMap;
use std::io::{Read, Stdin};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdinArgs {
    input: Option<String>,
    headers: HashMap<String, String>,
}

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

impl RequestArgs for StdinArgs {
    fn method(&self) -> Option<&String> {
        None
    }

    fn url(&self) -> Option<&crate::http::Url> {
        None
    }

    fn body(&self) -> Option<&String> {
        self.input.as_ref()
    }

    fn user(&self) -> Option<&String> {
        None
    }

    fn password(&self) -> Option<&String> {
        None
    }

    fn insecure(&self) -> bool {
        false
    }

    fn ca_cert(&self) -> Option<&String> {
        None
    }

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

pub fn ask_string(i: &Stdin, msg: &str) -> Result<String> {
    loop {
        if !msg.is_empty() {
            eprint!("{}", msg);
        }

        let mut buffer = String::new();
        i.read_line(&mut buffer)?;
        buffer.pop(); // remove last \n

        if !buffer.is_empty() {
            return Ok(buffer);
        }
    }
}

pub fn ask_binary(i: &Stdin, msg: &str) -> Result<bool> {
    let buffer = ask_string(i, msg)?;
    Ok(buffer.to_lowercase() == "y" || buffer.to_lowercase() == "yes")
}

pub fn ask_path(i: &Stdin, msg: &str) -> Result<String> {
    loop {
        let str = ask_string(i, msg)?;
        let expand_str = shellexpand::tilde(str.as_str()).to_string();
        let expand_path = Path::new(&expand_str);
        if expand_path.exists() {
            return Ok(expand_path.to_str().unwrap().to_string());
        } else {
            eprintln!("Path not found: {}", str);
        }
    }
}
