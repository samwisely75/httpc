use std::collections::HashMap;
use std::io::{self, Write, IsTerminal};

use anyhow::{Context, Result};
use colored::*;
use rustyline::{DefaultEditor, error::ReadlineError};
use serde_json;

use crate::http::{HttpClient, HttpConnectionProfile, HttpRequestArgs};
use crate::ini::IniProfile;
use crate::url::{Url, UrlPath};

pub struct Repl {
    editor: DefaultEditor,
    profile: IniProfile,
    client: HttpClient,
    session_headers: HashMap<String, String>,
    verbose: bool,
}

#[derive(Debug)]
pub struct ReplCommand {
    method: String,
    url: Url,
    body: Option<String>,
    headers: HashMap<String, String>,
}

#[derive(Debug)]
pub enum Command {
    Http(ReplCommand),
    Special(SpecialCommand),
    Empty,
}

#[derive(Debug)]
pub enum SpecialCommand {
    Help,
    Exit,
    Clear,
    ShowHeaders,
    SetHeader { name: String, value: String },
    RemoveHeader { name: String },
    SwitchProfile { name: String },
    Verbose,
}

impl Repl {
    pub fn new(profile: IniProfile, verbose: bool) -> Result<Self> {
        let editor = DefaultEditor::new().context("Failed to create line editor")?;
        let client = HttpClient::new(&profile)?;
        
        Ok(Self {
            editor,
            profile,
            client,
            session_headers: HashMap::new(),
            verbose,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Only show welcome message in interactive mode
        if io::stdin().is_terminal() {
            self.print_welcome();
        }
        
        loop {
            match self.read_command() {
                Ok(Command::Http(cmd)) => {
                    if let Err(e) = self.execute_http_command(cmd).await {
                        eprintln!("{}: {}", "Error".red().bold(), e);
                    }
                }
                Ok(Command::Special(cmd)) => {
                    if let Err(e) = self.handle_special_command(cmd) {
                        eprintln!("{}: {}", "Error".red().bold(), e);
                    }
                }
                Ok(Command::Empty) => continue,
                Err(e) => {
                    // Handle EOF gracefully in non-interactive mode
                    if !io::stdin().is_terminal() && e.to_string() == "EOF" {
                        break;
                    }
                    eprintln!("{}: {}", "Error".red().bold(), e);
                    // Exit on any error in non-interactive mode
                    if !io::stdin().is_terminal() {
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }

    fn print_welcome(&self) {
        println!("{}", "Welcome to webly interactive mode!".green().bold());
        println!("Type {} for help, {} to exit.", "!help".cyan(), "!exit".cyan());
        println!("Press {} to exit anytime.", "Ctrl+C".cyan());
        println!("Enter HTTP commands like: {} {}", 
                 "GET".yellow(), 
                 "/api/users".blue());
        println!("For requests with body, press {} after the URL to enter body mode.",
                 "Enter".yellow());
        println!();
    }

    fn read_command(&mut self) -> Result<Command> {
        let line = if io::stdin().is_terminal() {
            // Interactive mode - use rustyline for history and editing
            let prompt = format!("{} ", "webly>".green().bold());
            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let _ = self.editor.add_history_entry(line.as_str());
                    line
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl+C pressed
                    println!("\n{}", "Goodbye!".green());
                    std::process::exit(0);
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl+D pressed
                    println!("\n{}", "Goodbye!".green());
                    std::process::exit(0);
                }
                Err(e) => {
                    anyhow::bail!("Failed to read input: {}", e);
                }
            }
        } else {
            // Non-interactive mode - read from stdin directly
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) => anyhow::bail!("EOF"),
                Ok(_) => line,
                Err(e) => anyhow::bail!("Failed to read input: {}", e),
            }
        };

        let line = line.trim();
        if line.is_empty() {
            return Ok(Command::Empty);
        }

        if line.starts_with('!') {
            return Ok(Command::Special(self.parse_special_command(line)?));
        }

        self.parse_http_command(line)
    }

    fn parse_special_command(&self, line: &str) -> Result<SpecialCommand> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        match parts[0] {
            "!help" | "!h" => Ok(SpecialCommand::Help),
            "!exit" | "!quit" | "!q" => Ok(SpecialCommand::Exit),
            "!clear" | "!c" => Ok(SpecialCommand::Clear),
            "!headers" => Ok(SpecialCommand::ShowHeaders),
            "!verbose" | "!v" => Ok(SpecialCommand::Verbose),
            "!set-header" | "!set" => {
                if parts.len() < 3 {
                    anyhow::bail!("Usage: !set-header <name> <value>");
                }
                Ok(SpecialCommand::SetHeader {
                    name: parts[1].to_string(),
                    value: parts[2..].join(" "),
                })
            }
            "!remove-header" | "!rm" => {
                if parts.len() < 2 {
                    anyhow::bail!("Usage: !remove-header <name>");
                }
                Ok(SpecialCommand::RemoveHeader {
                    name: parts[1].to_string(),
                })
            }
            "!profile" => {
                if parts.len() < 2 {
                    anyhow::bail!("Usage: !profile <name>");
                }
                Ok(SpecialCommand::SwitchProfile {
                    name: parts[1].to_string(),
                })
            }
            _ => anyhow::bail!("Unknown command: {}", parts[0]),
        }
    }

    fn parse_http_command(&mut self, line: &str) -> Result<Command> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() < 2 {
            anyhow::bail!("Usage: <METHOD> <URL> [body]");
        }

        let method = parts[0].to_uppercase();
        let url_str = parts[1];
        
        // Parse URL
        let url = Url::parse(url_str);

        // Check if this is a method that typically has a body
        let has_body = matches!(method.as_str(), "POST" | "PUT" | "PATCH");
        
        let body = if has_body {
            // Check if body is provided on the same line
            if parts.len() > 2 {
                Some(parts[2..].join(" "))
            } else {
                // Enter multi-line body mode
                self.read_body()?
            }
        } else {
            None
        };

        Ok(Command::Http(ReplCommand {
            method,
            url,
            body,
            headers: self.session_headers.clone(),
        }))
    }

    fn read_body(&mut self) -> Result<Option<String>> {
        println!("{}", "Enter body (press Ctrl+D when done, Ctrl+C to cancel):".yellow());
        
        let mut body = String::new();
        loop {
            let prompt = "> ";
            print!("{}", prompt);
            io::stdout().flush()?;
            
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) => break, // EOF (Ctrl+D)
                Ok(_) => {
                    body.push_str(&line);
                }
                Err(e) => {
                    anyhow::bail!("Error reading input: {}", e);
                }
            }
        }

        if body.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(body.trim().to_string()))
        }
    }

    async fn execute_http_command(&self, cmd: ReplCommand) -> Result<()> {
        // Check if we have a valid server endpoint for relative URLs
        if cmd.url.to_endpoint().is_none() && self.profile.server().is_none() {
            anyhow::bail!("Relative URL specified but no server configured in profile. Use absolute URL or configure a profile with server endpoint.");
        }
        
        let start_time = std::time::Instant::now();
        
        let response = self.client.request(&cmd).await?;
        
        let duration = start_time.elapsed();
        
        // Print status line
        let status_color = if response.status().is_success() {
            "green"
        } else if response.status().is_client_error() {
            "yellow"
        } else {
            "red"
        };
        
        if self.verbose {
            println!("{} {} {} ({}ms)", 
                     "HTTP".cyan().bold(),
                     response.status().as_u16().to_string().color(status_color).bold(),
                     response.status().canonical_reason().unwrap_or(""),
                     duration.as_millis());

            // Print headers in verbose mode
            if !response.headers().is_empty() {
                println!("{}", "Headers:".cyan());
                for (name, value) in response.headers() {
                    println!("  {}: {}", 
                             name.as_str().blue(),
                             value.to_str().unwrap_or("<invalid>"));
                }
            }
        } else {
            // In non-verbose mode, only show status if it's not successful
            if !response.status().is_success() {
                println!("{} {} {}", 
                         "HTTP".cyan().bold(),
                         response.status().as_u16().to_string().color(status_color).bold(),
                         response.status().canonical_reason().unwrap_or(""));
            }
        }

        // Print body
        if let Some(json) = response.json() {
            println!("{}", serde_json::to_string_pretty(json)?);
        } else if !response.body().is_empty() {
            println!("{}", response.body());
        }

        println!();
        Ok(())
    }

    fn handle_special_command(&mut self, cmd: SpecialCommand) -> Result<()> {
        match cmd {
            SpecialCommand::Help => {
                self.print_help();
            }
            SpecialCommand::Exit => {
                println!("{}", "Goodbye!".green());
                std::process::exit(0);
            }
            SpecialCommand::Clear => {
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush()?;
            }
            SpecialCommand::ShowHeaders => {
                if self.session_headers.is_empty() {
                    println!("{}", "No session headers set.".yellow());
                } else {
                    println!("{}", "Session headers:".cyan().bold());
                    for (name, value) in &self.session_headers {
                        println!("  {}: {}", name.blue(), value);
                    }
                }
            }
            SpecialCommand::SetHeader { name, value } => {
                self.session_headers.insert(name.to_lowercase(), value.clone());
                println!("{}: {} -> {}", "Header set".green(), name.blue(), value);
            }
            SpecialCommand::RemoveHeader { name } => {
                let key = name.to_lowercase();
                if self.session_headers.remove(&key).is_some() {
                    println!("{}: {}", "Header removed".green(), name.blue());
                } else {
                    println!("{}: {}", "Header not found".yellow(), name.blue());
                }
            }
            SpecialCommand::SwitchProfile { name } => {
                println!("{}: Profile switching to '{}' not implemented yet.", "Info".yellow(), name);
            }
            SpecialCommand::Verbose => {
                self.verbose = !self.verbose;
                let status = if self.verbose { "enabled" } else { "disabled" };
                println!("{}: Verbose mode {}", "Info".cyan(), status.yellow());
            }
        }
        Ok(())
    }

    fn print_help(&self) {
        println!("{}", "webly Interactive Mode Help".green().bold());
        println!();
        println!("{}", "HTTP Commands:".cyan().bold());
        println!("  {} {}           - Make a GET request", "GET".yellow(), "/api/users".blue());
        println!("  {} {}        - Make a POST request", "POST".yellow(), "/api/users".blue());
        println!("  {} {}         - Make a PUT request", "PUT".yellow(), "/api/users/1".blue());
        println!("  {} {}      - Make a DELETE request", "DELETE".yellow(), "/api/users/1".blue());
        println!();
        println!("{}", "Special Commands:".cyan().bold());
        println!("  {}                    - Show this help", "!help".yellow());
        println!("  {}                    - Exit webly", "!exit".yellow());
        println!("  {}                   - Clear screen", "!clear".yellow());
        println!("  {}                 - Show session headers", "!headers".yellow());
        println!("  {} {} {}  - Set a session header", "!set-header".yellow(), "name".blue(), "value".blue());
        println!("  {} {}     - Remove a session header", "!remove-header".yellow(), "name".blue());
        println!("  {}                 - Toggle verbose mode", "!verbose".yellow());
        println!();
        println!("{}", "Tips:".cyan().bold());
        println!("  - For POST/PUT/PATCH without inline body, press Enter to enter multi-line mode");
        println!("  - Use Ctrl+D to finish multi-line input, Ctrl+C to cancel");
        println!("  - Session headers persist across requests");
        println!("  - Use arrow keys to navigate command history");
        println!();
    }
}

impl HttpRequestArgs for ReplCommand {
    fn method(&self) -> Option<&String> {
        Some(&self.method)
    }

    fn url_path(&self) -> Option<&UrlPath> {
        self.url.to_url_path()
    }

    fn body(&self) -> Option<&String> {
        self.body.as_ref()
    }

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}
