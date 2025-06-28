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
            eprint!("{msg}");
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
            eprintln!("Path not found: {str}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_stdin_args_new_empty_when_tty() {
        // When stdin is a TTY (interactive terminal), StdinArgs should be empty
        // This test will work when run in a terminal
        let mut stdin = std::io::stdin();
        let result = StdinArgs::new(&mut stdin);

        if let Ok(args) = result {
            // If we're in a TTY, input should be None
            if atty::is(atty::Stream::Stdin) {
                assert_eq!(args.input, None);
                assert!(args.headers.is_empty());
            }
        }
    }

    #[test]
    fn test_stdin_args_implements_http_request_args() {
        let args = StdinArgs {
            input: Some("test body".to_string()),
            headers: HashMap::new(),
        };

        // Test HttpRequestArgs implementation
        assert_eq!(args.method(), None);
        assert_eq!(args.url_path(), None);
        assert_eq!(args.body(), Some(&"test body".to_string()));
        assert!(args.headers().is_empty());
    }

    #[test]
    fn test_stdin_args_with_body() {
        let args = StdinArgs {
            input: Some("request body content".to_string()),
            headers: HashMap::new(),
        };

        assert_eq!(args.body(), Some(&"request body content".to_string()));
    }

    #[test]
    fn test_stdin_args_without_body() {
        let args = StdinArgs {
            input: None,
            headers: HashMap::new(),
        };

        assert_eq!(args.body(), None);
    }

    #[test]
    fn test_stdin_args_debug_and_clone() {
        let args = StdinArgs {
            input: Some("test".to_string()),
            headers: HashMap::new(),
        };

        // Test Debug trait
        let debug_str = format!("{args:?}");
        assert!(debug_str.contains("StdinArgs"));

        // Test Clone trait
        let cloned = args.clone();
        assert_eq!(args, cloned);
    }

    #[test]
    fn test_ask_no_space_string_valid() {
        // Test would require mocking stdin input, which is complex
        // For now, we test the regex pattern used
        let re = regex::Regex::new(r"^[^\s\t]+$").unwrap();

        assert!(re.is_match("valid"));
        assert!(re.is_match("file.txt"));
        assert!(re.is_match("no-spaces"));
        assert!(!re.is_match("has spaces"));
        assert!(!re.is_match("has\ttab"));
        assert!(!re.is_match(""));
    }

    #[test]
    fn test_ask_binary_regex() {
        // Test the regex pattern for binary input
        let re = regex::Regex::new(r"^[YyNn]$").unwrap();

        assert!(re.is_match("Y"));
        assert!(re.is_match("y"));
        assert!(re.is_match("N"));
        assert!(re.is_match("n"));
        assert!(!re.is_match("yes"));
        assert!(!re.is_match("no"));
        assert!(!re.is_match(""));
        assert!(!re.is_match("maybe"));
    }

    #[test]
    fn test_ask_path_with_existing_file() {
        // Create a temporary file for testing
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let file_path = temp_file.path().to_str().unwrap();

        // Test that the path exists
        assert!(std::path::Path::new(file_path).exists());

        // The actual ask_path function would need mocked stdin, but we can test
        // the path validation logic separately
        let expanded = shellexpand::tilde(file_path).to_string();
        let path = std::path::Path::new(&expanded);
        assert!(path.exists());
    }

    #[test]
    fn test_tilde_expansion() {
        // Test tilde expansion functionality used in ask_path
        let home_path = "~/test";
        let expanded = shellexpand::tilde(home_path).to_string();

        // Should expand ~ to the home directory
        assert!(!expanded.starts_with('~'));
        assert!(expanded.len() > home_path.len());
    }

    #[test]
    fn test_regex_compilation() {
        // Test that various regex patterns compile correctly
        assert!(regex::Regex::new(r"^[^\s\t]+$").is_ok());
        assert!(regex::Regex::new(r"^[YyNn]$").is_ok());
        assert!(regex::Regex::new(r"\d+").is_ok());

        // Test invalid regex (unclosed character class)
        let invalid_regex = "[unclosed";
        assert!(regex::Regex::new(invalid_regex).is_err());
    }

    #[test]
    fn test_stdin_args_equality() {
        let args1 = StdinArgs {
            input: Some("test".to_string()),
            headers: HashMap::new(),
        };

        let args2 = StdinArgs {
            input: Some("test".to_string()),
            headers: HashMap::new(),
        };

        let args3 = StdinArgs {
            input: Some("different".to_string()),
            headers: HashMap::new(),
        };

        assert_eq!(args1, args2);
        assert_ne!(args1, args3);
    }

    #[test]
    fn test_stdin_args_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        let args = StdinArgs {
            input: Some("{}".to_string()),
            headers: headers.clone(),
        };

        assert_eq!(args.headers(), &headers);
        assert_eq!(
            args.headers().get("content-type"),
            Some(&"application/json".to_string())
        );
    }
}
