use std::io::{Read, Stdin};
use std::path::Path;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn read_stdin(i: &mut Stdin) -> Result<Option<String>> {
    let mut buffer = String::new();
    i.read_to_string(&mut buffer)?;

    if buffer.len() == 0 {
        Ok(None)
    } else {
        Ok(Some(buffer))
    }
}

pub fn ask_string(i: &Stdin, msg: &str) -> Result<String> {
    loop {
        if msg.len() > 0 {
            eprint!("{}", msg);
        }

        let mut buffer = String::new();
        i.read_line(&mut buffer)?;
        buffer.pop(); // remove last \n

        if buffer.len() > 0 {
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
