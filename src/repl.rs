use std::collections::HashMap;
use std::io::{self, Write};

use anyhow::Result;
use crossterm::{
    cursor::{self, SetCursorStyle},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

use crate::http::{HttpClient, HttpRequestArgs};
use crate::ini::IniProfile;
use crate::url::{Url, UrlPath};

#[derive(Debug, Clone, PartialEq)]
pub enum EditorMode {
    Normal,
    Insert,
    Command,
    Visual,
    VisualLine,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualSelection {
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

#[derive(Debug, Clone)]
pub struct Buffer {
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: usize,
}

#[derive(Debug, Clone)]
pub struct ResponseBuffer {
    content: String,
    lines: Vec<String>,
    scroll_offset: usize,
    cursor_line: usize,
    cursor_col: usize,
}

pub struct VimRepl {
    mode: EditorMode,
    buffer: Buffer,
    response_buffer: Option<ResponseBuffer>,
    current_pane: Pane,
    client: HttpClient,
    profile: IniProfile,
    session_headers: HashMap<String, String>,
    verbose: bool,
    terminal_size: (u16, u16),
    status_message: String,
    command_buffer: String,
    clipboard: String,
    visual_selection: Option<VisualSelection>,
    pending_g: bool,
    pending_ctrl_w: bool,
    last_response_status: Option<String>,
    pane_split_ratio: f64, // 0.0 to 1.0, represents input pane height ratio
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pane {
    Request,
    Response,
}

#[derive(Debug)]
pub struct ReplCommand {
    method: String,
    url: Url,
    body: Option<String>,
    headers: HashMap<String, String>,
}

impl Buffer {
    fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
        }
    }

    fn insert_char(&mut self, ch: char) {
        if self.cursor_line >= self.lines.len() {
            self.lines.push(String::new());
        }
        
        let line = &mut self.lines[self.cursor_line];
        if self.cursor_col <= line.len() {
            line.insert(self.cursor_col, ch);
            self.cursor_col += 1;
        }
    }

    fn delete_char(&mut self) {
        if self.cursor_line >= self.lines.len() {
            return;
        }
        
        let line = &mut self.lines[self.cursor_line];
        if self.cursor_col > 0 && self.cursor_col <= line.len() {
            line.remove(self.cursor_col - 1);
            self.cursor_col -= 1;
        } else if self.cursor_col == 0 && self.cursor_line > 0 {
            // At beginning of line, join with previous line
            let current_line = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&current_line);
        }
    }

    fn delete_char_at_cursor(&mut self) {
        if self.cursor_line >= self.lines.len() {
            return;
        }
        
        let line = &mut self.lines[self.cursor_line];
        if self.cursor_col < line.len() {
            line.remove(self.cursor_col);
        } else if self.cursor_line + 1 < self.lines.len() {
            // At end of line, join with next line
            let next_line = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next_line);
        }
    }

    fn new_line(&mut self) {
        if self.cursor_line >= self.lines.len() {
            self.lines.push(String::new());
        }
        
        let line = &mut self.lines[self.cursor_line];
        let remainder = line.split_off(self.cursor_col);
        self.cursor_line += 1;
        self.lines.insert(self.cursor_line, remainder);
        self.cursor_col = 0;
    }

    fn new_line_with_scroll(&mut self, visible_height: usize) {
        self.new_line();
        
        // Auto-scroll down if cursor goes below visible area
        if self.cursor_line >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor_line - visible_height + 1;
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
            self.cursor_col = self.cursor_col.min(line_len);
        }
    }

    fn move_cursor_up_with_scroll(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
            self.cursor_col = self.cursor_col.min(line_len);
            
            // Auto-scroll up if cursor goes above visible area
            if self.cursor_line < self.scroll_offset {
                self.scroll_offset = self.cursor_line;
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_line < self.lines.len() - 1 {
            self.cursor_line += 1;
            let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
            self.cursor_col = self.cursor_col.min(line_len);
        }
    }

    fn move_cursor_down_with_scroll(&mut self, visible_height: usize) {
        if self.cursor_line < self.lines.len() - 1 {
            self.cursor_line += 1;
            let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
            self.cursor_col = self.cursor_col.min(line_len);
            
            // Auto-scroll down if cursor goes below visible area
            if self.cursor_line >= self.scroll_offset + visible_height {
                self.scroll_offset = self.cursor_line - visible_height + 1;
            }
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            if self.cursor_col < line.len() {
                self.cursor_col += 1;
            }
        }
    }

    fn join_with_next_line(&mut self) {
        if self.cursor_line + 1 < self.lines.len() {
            let next_line = self.lines.remove(self.cursor_line + 1);
            // Add a space between lines if current line doesn't end with whitespace
            // and next line doesn't start with whitespace
            let current_line = &mut self.lines[self.cursor_line];
            if !current_line.is_empty() && !current_line.ends_with(' ') && 
               !next_line.is_empty() && !next_line.starts_with(' ') {
                current_line.push(' ');
            }
            current_line.push_str(&next_line);
        }
    }

    fn move_cursor_to_line_start(&mut self) {
        self.cursor_col = 0;
    }

    fn move_cursor_to_line_end(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            self.cursor_col = line.len();
        }
    }

    fn move_cursor_to_start(&mut self) {
        self.cursor_line = 0;
        self.cursor_col = 0;
    }

    fn move_cursor_to_end(&mut self) {
        if !self.lines.is_empty() {
            self.cursor_line = self.lines.len() - 1;
            self.cursor_col = self.lines[self.cursor_line].len();
        }
    }

    fn move_cursor_word_forward(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            let chars: Vec<char> = line.chars().collect();
            let mut pos = self.cursor_col;
            
            // Skip current word
            while pos < chars.len() && chars[pos].is_alphanumeric() {
                pos += 1;
            }
            
            // Skip whitespace
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }
            
            // If we're at the end of the line, move to next line
            if pos >= chars.len() && self.cursor_line < self.lines.len() - 1 {
                self.cursor_line += 1;
                self.cursor_col = 0;
            } else {
                self.cursor_col = pos;
            }
        }
    }

    fn move_cursor_word_backward(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            let chars: Vec<char> = line.chars().collect();
            let mut pos = self.cursor_col;
            
            if pos > 0 {
                pos -= 1;
                
                // Skip whitespace
                while pos > 0 && chars[pos].is_whitespace() {
                    pos -= 1;
                }
                
                // Skip current word
                while pos > 0 && chars[pos].is_alphanumeric() {
                    pos -= 1;
                }
                
                // If we stopped on a non-alphanumeric character, move one forward
                if pos > 0 && !chars[pos].is_alphanumeric() {
                    pos += 1;
                }
                
                self.cursor_col = pos;
            } else if self.cursor_line > 0 {
                // Move to end of previous line
                self.cursor_line -= 1;
                self.cursor_col = self.lines[self.cursor_line].len();
            }
        }
    }

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn scroll_down(&mut self, max_lines: usize) {
        if self.scroll_offset + max_lines < self.lines.len() {
            self.scroll_offset += 1;
        }
    }

    fn scroll_half_page_up(&mut self, half_page_size: usize) {
        let scroll_amount = half_page_size.min(self.scroll_offset);
        self.scroll_offset -= scroll_amount;
    }

    fn scroll_half_page_down(&mut self, half_page_size: usize, max_lines: usize) {
        let max_scroll = if self.lines.len() > max_lines {
            self.lines.len() - max_lines
        } else {
            0
        };
        let scroll_amount = half_page_size.min(max_scroll - self.scroll_offset);
        self.scroll_offset += scroll_amount;
    }

    fn delete_to_line_end(&mut self) -> String {
        if self.cursor_line >= self.lines.len() {
            return String::new();
        }
        
        let line = &mut self.lines[self.cursor_line];
        if self.cursor_col < line.len() {
            let deleted = line.split_off(self.cursor_col);
            deleted
        } else {
            String::new()
        }
    }

    fn get_text(&self) -> String {
        self.lines.join("\n")
    }

    fn yank_line(&self) -> String {
        self.lines.get(self.cursor_line).cloned().unwrap_or_default()
    }

    fn yank_to_line_end(&self) -> String {
        if self.cursor_line < self.lines.len() {
            if let Some(line) = self.lines.get(self.cursor_line) {
                if self.cursor_col < line.len() {
                    return line[self.cursor_col..].to_string();
                }
            }
        }
        String::new()
    }

    fn delete_line(&mut self) -> String {
        if self.lines.len() > 1 {
            let deleted = self.lines.remove(self.cursor_line);
            if self.cursor_line >= self.lines.len() {
                self.cursor_line = self.lines.len() - 1;
            }
            self.cursor_col = 0;
            deleted
        } else {
            let deleted = self.lines[0].clone();
            self.lines[0].clear();
            self.cursor_col = 0;
            deleted
        }
    }

    fn paste_line(&mut self, text: &str) {
        self.lines.insert(self.cursor_line + 1, text.to_string());
        self.cursor_line += 1;
        self.cursor_col = 0;
    }

    fn paste_line_above(&mut self, text: &str) {
        self.lines.insert(self.cursor_line, text.to_string());
        self.cursor_col = 0;
    }
}

impl ResponseBuffer {
    fn new(content: String) -> Self {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Self {
            content,
            lines,
            scroll_offset: 0,
            cursor_line: 0,
            cursor_col: 0,
        }
    }

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn scroll_down(&mut self, max_lines: usize) {
        if self.scroll_offset + max_lines < self.lines.len() {
            self.scroll_offset += 1;
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
            self.cursor_col = self.cursor_col.min(line_len);
            
            // Auto-scroll up if cursor goes above visible area
            if self.cursor_line < self.scroll_offset {
                self.scroll_offset = self.cursor_line;
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_line < self.lines.len().saturating_sub(1) {
            self.cursor_line += 1;
            let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
            self.cursor_col = self.cursor_col.min(line_len);
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            if self.cursor_col < line.len() {
                self.cursor_col += 1;
            }
        }
    }

    fn move_cursor_to_line_start(&mut self) {
        self.cursor_col = 0;
    }

    fn move_cursor_to_line_end(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            self.cursor_col = line.len();
        }
    }

    fn scroll_half_page_up(&mut self, half_page_size: usize) {
        let scroll_amount = half_page_size.min(self.scroll_offset);
        self.scroll_offset -= scroll_amount;
    }

    fn scroll_half_page_down(&mut self, half_page_size: usize, max_lines: usize) {
        let max_scroll = if self.lines.len() > max_lines {
            self.lines.len() - max_lines
        } else {
            0
        };
        let scroll_amount = half_page_size.min(max_scroll - self.scroll_offset);
        self.scroll_offset += scroll_amount;
    }
}

impl VimRepl {
    pub fn new(profile: IniProfile, verbose: bool) -> Result<Self> {
        let client = HttpClient::new(&profile)?;
        let terminal_size = terminal::size()?;

        Ok(Self {
            mode: EditorMode::Insert,
            buffer: Buffer::new(),
            response_buffer: None,
            current_pane: Pane::Request,
            client,
            profile,
            session_headers: HashMap::new(),
            verbose,
            terminal_size,
            status_message: "-- INSERT --".to_string(),
            command_buffer: String::new(),
            clipboard: String::new(),
            visual_selection: None,
            pending_g: false,
            pending_ctrl_w: false,
            last_response_status: None,
            pane_split_ratio: 0.5, // Start with 50/50 split
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Enable raw mode for terminal
        terminal::enable_raw_mode()?;
        
        // Clear screen and move cursor to top
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        
        // Set initial cursor style for insert mode
        execute!(io::stdout(), SetCursorStyle::BlinkingBar)?;
        
        let result = self.event_loop().await;
        
        // Clean up - restore default cursor
        execute!(io::stdout(), SetCursorStyle::DefaultUserShape)?;
        terminal::disable_raw_mode()?;
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        
        result
    }

    async fn event_loop(&mut self) -> Result<()> {
        loop {
            self.render()?;
            
            match event::read()? {
                Event::Key(key) => {
                    if self.handle_key_event(key).await? {
                        break;
                    }
                }
                Event::Resize(width, height) => {
                    self.terminal_size = (width, height);
                }
                _ => {}
            }
        }
        
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match self.mode {
            EditorMode::Normal => self.handle_normal_mode(key).await,
            EditorMode::Insert => self.handle_insert_mode(key).await,
            EditorMode::Command => self.handle_command_mode(key).await,
            EditorMode::Visual | EditorMode::VisualLine => self.handle_visual_mode(key),
        }
    }

    async fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<bool> {
        // Reset pending flags for most commands
        let reset_pending_g = match key.code {
            KeyCode::Char('g') => false,
            _ => true,
        };
        
        let reset_pending_ctrl_w = match key.code {
            KeyCode::Char('w') | KeyCode::Char('h') | KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('l') => {
                // Only reset if we're not in a pending Ctrl+W state
                !self.pending_ctrl_w
            }
            _ => true,
        };
        
        if reset_pending_g {
            self.pending_g = false;
        }
        
        if reset_pending_ctrl_w {
            self.pending_ctrl_w = false;
        }

        // Handle Ctrl+W window navigation (vim-style)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('w') {
            self.pending_ctrl_w = true;
            self.status_message = "Ctrl+W pressed. Press w/h/j/k/l for window navigation".to_string();
            return Ok(false);
        }
        
        // Handle second step of Ctrl+W commands
        if self.pending_ctrl_w {
            match key.code {
                KeyCode::Char('w') => {
                    // Ctrl+W w - switch to next window
                    self.current_pane = match self.current_pane {
                        Pane::Request => Pane::Response,
                        Pane::Response => Pane::Request,
                    };
                    self.status_message = format!("Switched to {} pane", 
                        match self.current_pane {
                            Pane::Request => "request",
                            Pane::Response => "response",
                        });
                    self.pending_ctrl_w = false;
                    return Ok(false);
                }
                KeyCode::Char('h') => {
                    // Ctrl+W h - move to left window (Request pane)
                    self.current_pane = Pane::Request;
                    self.status_message = "Switched to request pane".to_string();
                    self.pending_ctrl_w = false;
                    return Ok(false);
                }
                KeyCode::Char('l') => {
                    // Ctrl+W l - move to right window (Response pane)
                    self.current_pane = Pane::Response;
                    self.status_message = "Switched to response pane".to_string();
                    self.pending_ctrl_w = false;
                    return Ok(false);
                }
                KeyCode::Char('j') | KeyCode::Char('k') => {
                    // Ctrl+W j/k - in our case, just toggle between panes
                    self.current_pane = match self.current_pane {
                        Pane::Request => Pane::Response,
                        Pane::Response => Pane::Request,
                    };
                    self.status_message = format!("Switched to {} pane", 
                        match self.current_pane {
                            Pane::Request => "request",
                            Pane::Response => "response",
                        });
                    self.pending_ctrl_w = false;
                    return Ok(false);
                }
                KeyCode::Esc => {
                    // Cancel Ctrl+W command
                    self.pending_ctrl_w = false;
                    self.status_message = "Cancelled window command".to_string();
                    return Ok(false);
                }
                _ => {
                    // Invalid Ctrl+W command
                    self.pending_ctrl_w = false;
                    self.status_message = "Invalid window command".to_string();
                    return Ok(false);
                }
            }
        }

        // Handle Ctrl+U and Ctrl+D for scrolling in normal mode
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('u') => {
                    if self.current_pane == Pane::Request {
                        let half_page = self.get_request_pane_height() / 2;
                        self.buffer.scroll_half_page_up(half_page);
                        self.status_message = "Scrolled up half page".to_string();
                    } else if self.response_buffer.is_some() {
                        let half_page = self.get_response_pane_height() / 2;
                        if let Some(ref mut response) = self.response_buffer {
                            for _ in 0..half_page {
                                response.scroll_up();
                            }
                        }
                        self.status_message = "Scrolled up half page".to_string();
                    }
                    return Ok(false);
                }
                KeyCode::Char('d') => {
                    if self.current_pane == Pane::Request {
                        let half_page = self.get_request_pane_height() / 2;
                        let max_lines = self.get_request_pane_height();
                        self.buffer.scroll_half_page_down(half_page, max_lines);
                        self.status_message = "Scrolled down half page".to_string();
                    } else if self.response_buffer.is_some() {
                        let half_page = self.get_response_pane_height() / 2;
                        let max_lines = self.get_response_pane_height();
                        if let Some(ref mut response) = self.response_buffer {
                            for _ in 0..half_page {
                                response.scroll_down(max_lines);
                            }
                        }
                        self.status_message = "Scrolled down half page".to_string();
                    }
                    return Ok(false);
                }
                KeyCode::Char('k') => {
                    // Ctrl+K - Boundary control (upward)
                    if self.response_buffer.is_some() {
                        if self.current_pane == Pane::Request {
                            // Input pane: shrink
                            self.shrink_input_pane();
                        } else {
                            // Output pane: expand
                            self.expand_output_pane();
                        }
                    }
                    return Ok(false);
                }
                KeyCode::Char('j') => {
                    // Ctrl+J - Boundary control (downward)
                    if self.response_buffer.is_some() {
                        if self.current_pane == Pane::Request {
                            // Input pane: expand
                            self.expand_input_pane();
                        } else {
                            // Output pane: shrink
                            self.shrink_output_pane();
                        }
                    }
                    return Ok(false);
                }
                KeyCode::Char('m') => {
                    // Ctrl+M - Maximize current pane
                    if self.response_buffer.is_some() {
                        if self.current_pane == Pane::Request {
                            // Maximize input pane
                            self.maximize_input_pane();
                        } else {
                            // Maximize output pane
                            self.maximize_output_pane();
                        }
                    }
                    return Ok(false);
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('i') => {
                // Only allow insert mode in request pane
                if self.current_pane == Pane::Request {
                    self.mode = EditorMode::Insert;
                    self.status_message = "-- INSERT --".to_string();
                    // Set cursor to line style for insert mode
                    execute!(io::stdout(), SetCursorStyle::BlinkingBar)?;
                } else {
                    self.status_message = "Insert mode not allowed in response pane".to_string();
                }
            }
            KeyCode::Char('I') => {
                // Insert at beginning of line - only in request pane
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_line_start();
                    self.mode = EditorMode::Insert;
                    self.status_message = "-- INSERT --".to_string();
                    execute!(io::stdout(), SetCursorStyle::BlinkingBar)?;
                } else {
                    self.status_message = "Insert mode not allowed in response pane".to_string();
                }
            }
            KeyCode::Char('A') => {
                // Append at end of line - only in request pane
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_line_end();
                    self.mode = EditorMode::Insert;
                    self.status_message = "-- INSERT --".to_string();
                    execute!(io::stdout(), SetCursorStyle::BlinkingBar)?;
                } else {
                    self.status_message = "Insert mode not allowed in response pane".to_string();
                }
            }
            KeyCode::Char(':') => {
                self.mode = EditorMode::Command;
                self.command_buffer.clear();
                self.status_message = ":".to_string();
            }
            KeyCode::Char('J') => {
                // Shift+J - join with next line
                if self.current_pane == Pane::Request {
                    self.buffer.join_with_next_line();
                    self.status_message = "Lines joined".to_string();
                }
            }
            KeyCode::Char('U') => {
                // Shift+U - scroll up half page (vim-style)
                if self.current_pane == Pane::Request {
                    let half_page = self.get_request_pane_height() / 2;
                    self.buffer.scroll_half_page_up(half_page);
                    self.status_message = "Scrolled up half page".to_string();
                } else if self.response_buffer.is_some() {
                    let half_page = self.get_response_pane_height() / 2;
                    if let Some(ref mut response) = self.response_buffer {
                        for _ in 0..half_page {
                            response.scroll_up();
                        }
                    }
                    self.status_message = "Scrolled up half page".to_string();
                }
            }
            KeyCode::Char('D') => {
                // Shift+D in normal mode - scroll down half page (vim-style)
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    if self.current_pane == Pane::Request {
                        let half_page = self.get_request_pane_height() / 2;
                        let max_lines = self.get_request_pane_height();
                        self.buffer.scroll_half_page_down(half_page, max_lines);
                        self.status_message = "Scrolled down half page".to_string();
                    } else if self.response_buffer.is_some() {
                        let half_page = self.get_response_pane_height() / 2;
                        let max_lines = self.get_response_pane_height();
                        if let Some(ref mut response) = self.response_buffer {
                            for _ in 0..half_page {
                                response.scroll_down(max_lines);
                            }
                        }
                        self.status_message = "Scrolled down half page".to_string();
                    }
                } else {
                    // Regular D - Delete from cursor to end of line
                    if self.current_pane == Pane::Request {
                        let deleted = self.buffer.delete_to_line_end();
                        if !deleted.is_empty() {
                            self.clipboard = deleted;
                            self.status_message = "Deleted to end of line".to_string();
                        }
                    }
                }
            }
            KeyCode::Char('b') => {
                // Move cursor backward by word
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_word_backward();
                }
            }
            KeyCode::Char('w') => {
                // Move cursor forward by word
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_word_forward();
                }
            }
            KeyCode::Char('g') => {
                // Handle 'gg' command
                if self.current_pane == Pane::Request {
                    if self.pending_g {
                        // Second 'g' - go to start of buffer
                        self.buffer.move_cursor_to_start();
                        self.status_message = "Start of buffer".to_string();
                        self.pending_g = false;
                    } else {
                        // First 'g' - wait for second 'g'
                        self.pending_g = true;
                        self.status_message = "Press 'g' again to go to start of buffer".to_string();
                    }
                }
            }
            KeyCode::Char('G') => {
                // Go to end of buffer
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_end();
                    self.status_message = "End of buffer".to_string();
                }
            }
            KeyCode::Char('v') => {
                // Start visual mode
                if self.current_pane == Pane::Request {
                    self.mode = EditorMode::Visual;
                    self.visual_selection = Some(VisualSelection {
                        start_line: self.buffer.cursor_line,
                        start_col: self.buffer.cursor_col,
                        end_line: self.buffer.cursor_line,
                        end_col: self.buffer.cursor_col,
                    });
                    self.status_message = "-- VISUAL --".to_string();
                }
            }
            KeyCode::Char('V') => {
                // Start visual line mode
                if self.current_pane == Pane::Request {
                    self.mode = EditorMode::VisualLine;
                    self.visual_selection = Some(VisualSelection {
                        start_line: self.buffer.cursor_line,
                        start_col: 0,
                        end_line: self.buffer.cursor_line,
                        end_col: self.buffer.lines.get(self.buffer.cursor_line).map_or(0, |l| l.len()),
                    });
                    self.status_message = "-- VISUAL LINE --".to_string();
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_left();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_left();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.current_pane == Pane::Request {
                    let old_cursor_line = self.buffer.cursor_line;
                    self.buffer.move_cursor_down();
                    
                    // If cursor didn't move (at bottom), scroll down instead
                    if self.buffer.cursor_line == old_cursor_line && self.buffer.cursor_line == self.buffer.lines.len().saturating_sub(1) {
                        let max_lines = self.get_request_pane_height();
                        self.buffer.scroll_down(max_lines);
                        self.status_message = "Scrolled down one line".to_string();
                    }
                } else if self.response_buffer.is_some() {
                    let visible_height = self.get_response_pane_height();
                    if let Some(ref mut response) = self.response_buffer {
                        let old_cursor_line = response.cursor_line;
                        response.move_cursor_down();
                        
                        // If cursor didn't move (at bottom), scroll down instead
                        if response.cursor_line == old_cursor_line && response.cursor_line == response.lines.len().saturating_sub(1) {
                            response.scroll_down(visible_height);
                            self.status_message = "Scrolled down one line".to_string();
                        } else {
                            // Auto-scroll down if cursor goes below visible area
                            if response.cursor_line >= response.scroll_offset + visible_height {
                                response.scroll_offset = response.cursor_line - visible_height + 1;
                            }
                        }
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.current_pane == Pane::Request {
                    let old_cursor_line = self.buffer.cursor_line;
                    self.buffer.move_cursor_up();
                    
                    // If cursor didn't move (at top), scroll up instead
                    if self.buffer.cursor_line == old_cursor_line && self.buffer.cursor_line == 0 {
                        self.buffer.scroll_up();
                        self.status_message = "Scrolled up one line".to_string();
                    }
                } else if self.response_buffer.is_some() {
                    if let Some(ref mut response) = self.response_buffer {
                        let old_cursor_line = response.cursor_line;
                        response.move_cursor_up();
                        
                        // If cursor didn't move (at top), scroll up instead
                        if response.cursor_line == old_cursor_line && response.cursor_line == 0 {
                            response.scroll_up();
                            self.status_message = "Scrolled up one line".to_string();
                        }
                    }
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_right();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_right();
                }
            }
            KeyCode::Tab => {
                self.current_pane = match self.current_pane {
                    Pane::Request => Pane::Response,
                    Pane::Response => Pane::Request,
                };
                self.status_message = format!("Switched to {} pane", 
                    match self.current_pane {
                        Pane::Request => "request",
                        Pane::Response => "response",
                    });
            }
            KeyCode::Enter => {
                if self.current_pane == Pane::Request {
                    self.execute_request().await?;
                }
            }
            KeyCode::Char('y') => {
                // Start yank operation - for now just yank current line
                if self.current_pane == Pane::Request {
                    self.clipboard = self.buffer.yank_line();
                    self.status_message = "Line yanked".to_string();
                } else if let Some(ref response) = self.response_buffer {
                    let line_idx = response.scroll_offset;
                    if let Some(line) = response.lines.get(line_idx) {
                        self.clipboard = line.clone();
                        self.status_message = "Line yanked from response".to_string();
                    }
                }
            }
            KeyCode::Char('Y') => {
                // Yank from cursor to end of line
                if self.current_pane == Pane::Request {
                    self.clipboard = self.buffer.yank_to_line_end();
                    self.status_message = "Yanked to end of line".to_string();
                }
            }
            KeyCode::Char('p') => {
                if self.current_pane == Pane::Request && !self.clipboard.is_empty() {
                    self.buffer.paste_line(&self.clipboard);
                    self.status_message = "Line pasted".to_string();
                }
            }
            KeyCode::Char('P') => {
                // Paste above current line
                if self.current_pane == Pane::Request && !self.clipboard.is_empty() {
                    self.buffer.paste_line_above(&self.clipboard);
                    self.status_message = "Line pasted above".to_string();
                }
            }
            KeyCode::Char('d') => {
                if self.current_pane == Pane::Request {
                    self.clipboard = self.buffer.delete_line();
                    self.status_message = "Line deleted".to_string();
                }
            }
            KeyCode::Char('x') => {
                if self.current_pane == Pane::Request {
                    self.buffer.delete_char_at_cursor();
                    self.status_message = "Character deleted".to_string();
                }
            }
            KeyCode::Delete => {
                if self.current_pane == Pane::Request {
                    self.buffer.delete_char_at_cursor();
                }
            }
            KeyCode::Backspace => {
                if self.current_pane == Pane::Request {
                    self.buffer.delete_char();
                }
            }
            KeyCode::Esc => {
                self.status_message = "".to_string();
            }
            KeyCode::PageUp => {
                if self.current_pane == Pane::Request {
                    let half_page = self.get_request_pane_height() / 2;
                    self.buffer.scroll_half_page_up(half_page);
                    self.status_message = "Scrolled up".to_string();
                } else if self.response_buffer.is_some() {
                    let half_page = self.get_response_pane_height() / 2;
                    if let Some(ref mut response) = self.response_buffer {
                        response.scroll_half_page_up(half_page);
                    }
                    self.status_message = "Scrolled up".to_string();
                }
            }
            KeyCode::PageDown => {
                if self.current_pane == Pane::Request {
                    let half_page = self.get_request_pane_height() / 2;
                    let max_lines = self.get_request_pane_height();
                    self.buffer.scroll_half_page_down(half_page, max_lines);
                    self.status_message = "Scrolled down".to_string();
                } else if self.response_buffer.is_some() {
                    let half_page = self.get_response_pane_height() / 2;
                    let max_lines = self.get_response_pane_height();
                    if let Some(ref mut response) = self.response_buffer {
                        response.scroll_half_page_down(half_page, max_lines);
                    }
                    self.status_message = "Scrolled down".to_string();
                }
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_insert_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.status_message = "".to_string();
                // Set cursor to block style for normal mode
                execute!(io::stdout(), SetCursorStyle::BlinkingBlock)?;
            }
            KeyCode::Enter => {
                // Check for Ctrl+Enter to execute request
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Execute request in insert mode
                    if self.current_pane == Pane::Request {
                        self.execute_request().await?;
                    }
                } else {
                    let visible_height = self.get_request_pane_height();
                    self.buffer.new_line_with_scroll(visible_height);
                }
            }
            KeyCode::Char(ch) => {
                // Handle Ctrl+character combinations
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match ch {
                        'm' => {
                            // Ctrl+M - Maximize current pane or execute request
                            if self.current_pane == Pane::Request {
                                if self.response_buffer.is_some() {
                                    // Maximize input pane
                                    self.maximize_input_pane();
                                } else {
                                    // Execute request if no response buffer
                                    self.execute_request().await?;
                                }
                            }
                        }
                        'j' => {
                            // Ctrl+J - Boundary control (downward) or execute request
                            if self.current_pane == Pane::Request {
                                if self.response_buffer.is_some() {
                                    // Expand input pane
                                    self.expand_input_pane();
                                } else {
                                    // Execute request if no response buffer
                                    self.execute_request().await?;
                                }
                            }
                        }
                        'd' => {
                            // Ctrl+D in insert mode - scroll down half page
                            if self.current_pane == Pane::Request {
                                let half_page = self.get_request_pane_height() / 2;
                                let max_lines = self.get_request_pane_height();
                                self.buffer.scroll_half_page_down(half_page, max_lines);
                            }
                        }
                        'u' => {
                            // Ctrl+U in insert mode - scroll up half page
                            if self.current_pane == Pane::Request {
                                let half_page = self.get_request_pane_height() / 2;
                                self.buffer.scroll_half_page_up(half_page);
                            }
                        }
                        'k' => {
                            // Ctrl+K - Boundary control (upward) in insert mode
                            if self.current_pane == Pane::Request && self.response_buffer.is_some() {
                                // Shrink input pane
                                self.shrink_input_pane();
                            }
                        }
                        _ => {
                            // Regular character with Ctrl - insert as normal for most cases
                            // But filter out common control characters that shouldn't be inserted
                            if ch.is_ascii_control() {
                                // Don't insert control characters
                            } else {
                                self.buffer.insert_char(ch);
                            }
                        }
                    }
                } else {
                    self.buffer.insert_char(ch);
                }
            }
            KeyCode::Backspace => {
                self.buffer.delete_char();
            }
            KeyCode::Delete => {
                self.buffer.delete_char_at_cursor();
            }
            KeyCode::Left => {
                self.buffer.move_cursor_left();
            }
            KeyCode::Right => {
                self.buffer.move_cursor_right();
            }
            KeyCode::Up => {
                self.buffer.move_cursor_up_with_scroll();
            }
            KeyCode::Down => {
                let visible_height = self.get_request_pane_height();
                self.buffer.move_cursor_down_with_scroll(visible_height);
            }
            KeyCode::PageUp => {
                if self.current_pane == Pane::Request {
                    let half_page = self.get_request_pane_height() / 2;
                    self.buffer.scroll_half_page_up(half_page);
                }
            }
            KeyCode::PageDown => {
                if self.current_pane == Pane::Request {
                    let half_page = self.get_request_pane_height() / 2;
                    let max_lines = self.get_request_pane_height();
                    self.buffer.scroll_half_page_down(half_page, max_lines);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    async fn handle_command_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.status_message = "".to_string();
                self.command_buffer.clear();
            }
            KeyCode::Char(ch) => {
                self.command_buffer.push(ch);
                self.status_message = format!(":{}", self.command_buffer);
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
                self.status_message = format!(":{}", self.command_buffer);
            }
            KeyCode::Enter => {
                let should_quit = self.execute_command().await?;
                self.mode = EditorMode::Normal;
                self.command_buffer.clear();
                return Ok(should_quit);
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_visual_mode(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.mode = EditorMode::Normal;
                self.status_message = "".to_string();
                self.visual_selection = None;
            }
            KeyCode::Char('v') => {
                // Toggle back to normal mode
                self.mode = EditorMode::Normal;
                self.status_message = "".to_string();
                self.visual_selection = None;
            }
            KeyCode::Char('V') => {
                // Toggle to visual line mode
                if self.mode == EditorMode::Visual {
                    self.mode = EditorMode::VisualLine;
                    self.status_message = "-- VISUAL LINE --".to_string();
                    if let Some(ref mut sel) = self.visual_selection {
                        sel.start_col = 0;
                        sel.end_col = self.buffer.lines.get(self.buffer.cursor_line).map_or(0, |l| l.len());
                    }
                } else {
                    self.mode = EditorMode::Visual;
                    self.status_message = "-- VISUAL --".to_string();
                    if let Some(ref mut sel) = self.visual_selection {
                        sel.end_col = self.buffer.cursor_col;
                    }
                }
            }
            KeyCode::Char('y') => {
                // Yank selection
                if let Some(ref selection) = self.visual_selection {
                    let yanked_text = self.get_selected_text(selection);
                    self.clipboard = yanked_text;
                    self.status_message = "Selection yanked".to_string();
                }
                self.mode = EditorMode::Normal;
                self.visual_selection = None;
            }
            KeyCode::Char('d') => {
                // Delete selection
                if let Some(selection) = self.visual_selection.take() {
                    let deleted_text = self.delete_selected_text(&selection);
                    self.clipboard = deleted_text;
                    self.status_message = "Selection deleted".to_string();
                }
                self.mode = EditorMode::Normal;
            }
            // Movement commands that update selection
            KeyCode::Char('h') | KeyCode::Left => {
                self.buffer.move_cursor_left();
                self.update_visual_selection();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let visible_height = self.get_request_pane_height();
                self.buffer.move_cursor_down_with_scroll(visible_height);
                self.update_visual_selection();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.buffer.move_cursor_up_with_scroll();
                self.update_visual_selection();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.buffer.move_cursor_right();
                self.update_visual_selection();
            }
            KeyCode::Char('w') => {
                self.buffer.move_cursor_word_forward();
                self.update_visual_selection();
            }
            KeyCode::Char('b') => {
                self.buffer.move_cursor_word_backward();
                self.update_visual_selection();
            }
            KeyCode::Char('G') => {
                self.buffer.move_cursor_to_end();
                self.update_visual_selection();
            }
            _ => {}
        }
        Ok(false)
    }

    fn update_visual_selection(&mut self) {
        if let Some(ref mut selection) = self.visual_selection {
            selection.end_line = self.buffer.cursor_line;
            if self.mode == EditorMode::VisualLine {
                selection.end_col = self.buffer.lines.get(self.buffer.cursor_line).map_or(0, |l| l.len());
            } else {
                selection.end_col = self.buffer.cursor_col;
            }
        }
    }

    fn get_selected_text(&self, selection: &VisualSelection) -> String {
        let start_line = selection.start_line.min(selection.end_line);
        let end_line = selection.start_line.max(selection.end_line);
        let start_col = if selection.start_line == start_line { selection.start_col } else { selection.end_col };
        let end_col = if selection.end_line == end_line { selection.end_col } else { selection.start_col };

        if start_line == end_line {
            // Single line selection
            if let Some(line) = self.buffer.lines.get(start_line) {
                let start_idx = start_col.min(end_col);
                let end_idx = start_col.max(end_col);
                return line[start_idx..end_idx.min(line.len())].to_string();
            }
        } else {
            // Multi-line selection
            let mut result = String::new();
            for (i, line) in self.buffer.lines.iter().enumerate().skip(start_line).take(end_line - start_line + 1) {
                if i == start_line {
                    result.push_str(&line[start_col..]);
                } else if i == end_line {
                    result.push_str(&line[..end_col.min(line.len())]);
                } else {
                    result.push_str(line);
                }
                if i < end_line {
                    result.push('\n');
                }
            }
            return result;
        }
        String::new()
    }

    fn delete_selected_text(&mut self, selection: &VisualSelection) -> String {
        let deleted_text = self.get_selected_text(selection);
        
        let start_line = selection.start_line.min(selection.end_line);
        let end_line = selection.start_line.max(selection.end_line);
        let start_col = if selection.start_line == start_line { selection.start_col } else { selection.end_col };
        let end_col = if selection.end_line == end_line { selection.end_col } else { selection.start_col };

        if start_line == end_line {
            // Single line deletion
            if let Some(line) = self.buffer.lines.get_mut(start_line) {
                let start_idx = start_col.min(end_col);
                let end_idx = start_col.max(end_col);
                line.drain(start_idx..end_idx.min(line.len()));
                self.buffer.cursor_col = start_idx;
            }
        } else {
            // Multi-line deletion
            if let Some(first_line) = self.buffer.lines.get_mut(start_line) {
                first_line.truncate(start_col);
            }
            if let Some(last_line) = self.buffer.lines.get(end_line) {
                let remainder = last_line[end_col.min(last_line.len())..].to_string();
                if let Some(first_line) = self.buffer.lines.get_mut(start_line) {
                    first_line.push_str(&remainder);
                }
            }
            // Remove lines in between
            for _ in start_line + 1..=end_line {
                if start_line + 1 < self.buffer.lines.len() {
                    self.buffer.lines.remove(start_line + 1);
                }
            }
            self.buffer.cursor_line = start_line;
            self.buffer.cursor_col = start_col;
        }
        
        deleted_text
    }

    async fn execute_command(&mut self) -> Result<bool> {
        match self.command_buffer.as_str() {
            "q" | "quit" => {
                if self.current_pane == Pane::Response {
                    // Hide output pane and switch to request pane
                    self.response_buffer = None;
                    self.current_pane = Pane::Request;
                    self.pane_split_ratio = 1.0; // Give full space to input pane
                    self.status_message = "Output pane hidden".to_string();
                } else {
                    self.status_message = "Goodbye!".to_string();
                    return Ok(true);
                }
            }
            "w" | "write" => {
                self.execute_request().await?;
            }
            "wq" => {
                self.execute_request().await?;
                return Ok(true);
            }
            "clear" => {
                self.response_buffer = None;
                self.status_message = "Response cleared".to_string();
            }
            "verbose" => {
                self.verbose = !self.verbose;
                self.status_message = format!("Verbose mode: {}", if self.verbose { "on" } else { "off" });
            }
            _ => {
                if self.command_buffer.starts_with("set ") {
                    let parts: Vec<&str> = self.command_buffer.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        let key = parts[1].to_string();
                        let value = parts[2].to_string();
                        self.session_headers.insert(key.clone(), value.clone());
                        self.status_message = format!("Header set: {} = {}", key, value);
                    }
                } else {
                    self.status_message = format!("Unknown command: {}", self.command_buffer);
                }
            }
        }
        Ok(false)
    }

    async fn execute_request(&mut self) -> Result<()> {
        let text = self.buffer.get_text();
        let lines: Vec<&str> = text.lines().collect();
        
        if lines.is_empty() || lines[0].trim().is_empty() {
            self.status_message = "No request to execute".to_string();
            return Ok(());
        }
        
        // Parse first line as method and URL
        let parts: Vec<&str> = lines[0].split_whitespace().collect();
        if parts.len() < 2 {
            self.status_message = "Invalid request format. Use: METHOD URL".to_string();
            return Ok(());
        }
        
        let method = parts[0].to_uppercase();
        let url_str = parts[1];
        let url = Url::parse(url_str);
        
        // Rest of the lines become the body
        let body = if lines.len() > 1 {
            Some(lines[1..].join("\n"))
        } else {
            None
        };
        
        let cmd = ReplCommand {
            method,
            url,
            body,
            headers: self.session_headers.clone(),
        };
        
        self.status_message = "Executing request...".to_string();
        
        match self.client.request(&cmd).await {
            Ok(response) => {
                let status = response.status();
                let headers = response.headers();
                let body = response.body();
                
                // Store the response status for display in status bar
                self.last_response_status = Some(format!("HTTP {} {}", 
                    status.as_u16(), 
                    status.canonical_reason().unwrap_or("")
                ));
                
                let mut response_text = String::new();
                response_text.push_str(&format!("HTTP {} {}\n", status.as_u16(), status.canonical_reason().unwrap_or("")));
                
                if self.verbose {
                    response_text.push_str("Headers:\n");
                    for (name, value) in headers {
                        response_text.push_str(&format!("  {}: {}\n", name.as_str(), value.to_str().unwrap_or("<invalid>")));
                    }
                    response_text.push('\n');
                }
                
                if let Some(json) = response.json() {
                    response_text.push_str(&serde_json::to_string_pretty(json).unwrap_or_else(|_| "Invalid JSON".to_string()));
                } else if !body.is_empty() {
                    response_text.push_str(body);
                }
                
                self.response_buffer = Some(ResponseBuffer::new(response_text));
                // Restore default split ratio when new response is generated
                if self.pane_split_ratio >= 1.0 {
                    self.pane_split_ratio = 0.5;
                }
                self.status_message = format!("Request completed: {}", status.as_u16());
            }
            Err(e) => {
                self.last_response_status = Some("Error".to_string());
                self.status_message = format!("Request failed: {}", e);
                self.response_buffer = Some(ResponseBuffer::new(format!("Error: {}", e)));
                // Restore default split ratio when new response is generated
                if self.pane_split_ratio >= 1.0 {
                    self.pane_split_ratio = 0.5;
                }
            }
        }
        
        Ok(())
    }

    fn render(&self) -> Result<()> {
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        
        let height = self.terminal_size.1 as usize;
        let width = self.terminal_size.0 as usize;
        
        // Calculate pane sizes using dynamic split ratio
        let request_height = if self.response_buffer.is_some() { 
            self.get_request_pane_height() 
        } else { 
            height - 1 // Full height minus status line
        };
        let response_height = if self.response_buffer.is_some() { 
            self.get_response_pane_height() 
        } else { 
            0 
        };
        
        // Render request pane (top)
        self.render_request_pane(request_height, width)?;
        
        // Render horizontal separator only if there's a response buffer
        if self.response_buffer.is_some() {
            execute!(io::stdout(), cursor::MoveTo(0, request_height as u16))?;
            execute!(io::stdout(), SetForegroundColor(Color::Cyan))?;
            print!("{}", "─".repeat(width));
            execute!(io::stdout(), ResetColor)?;
        }
        
        // Render response pane (bottom)
        if let Some(ref response) = self.response_buffer {
            self.render_response_pane(response, response_height, width, request_height + 1)?;
        }
        
        // Render status line
        self.render_status_line(height - 1, width)?;
        
        // Position cursor
        if self.current_pane == Pane::Request {
            let cursor_y = self.buffer.cursor_line.saturating_sub(self.buffer.scroll_offset);
            if cursor_y < request_height {
                // Calculate line number width and adjust cursor position
                let max_line_num = self.buffer.lines.len();
                let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
                let cursor_x = line_num_width + 1 + self.buffer.cursor_col; // +1 for space separator
                
                execute!(io::stdout(), cursor::MoveTo(
                    cursor_x as u16, 
                    cursor_y as u16
                ))?;
            }
        } else if self.current_pane == Pane::Response && self.response_buffer.is_some() {
            // Show cursor in response pane for navigation
            if let Some(ref response) = self.response_buffer {
                let cursor_y = response.cursor_line.saturating_sub(response.scroll_offset);
                if cursor_y < response_height {
                    // Calculate line number width and adjust cursor position
                    let max_line_num = response.lines.len();
                    let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
                    let cursor_x = line_num_width + 1 + response.cursor_col; // +1 for space separator
                    
                    execute!(io::stdout(), cursor::MoveTo(
                        cursor_x as u16,
                        (request_height + 1 + cursor_y) as u16
                    ))?;
                }
            }
        }
        
        io::stdout().flush()?;
        Ok(())
    }

    fn render_request_pane(&self, height: usize, width: usize) -> Result<()> {
        let active = self.current_pane == Pane::Request;
        
        // Calculate line number width based on total lines
        let max_line_num = self.buffer.lines.len();
        let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
        let content_width = width.saturating_sub(line_num_width + 1); // +1 for space separator
        
        for i in 0..height {
            execute!(io::stdout(), cursor::MoveTo(0, i as u16))?;
            
            let line_idx = i + self.buffer.scroll_offset;
            
            // Render line number
            if line_idx < self.buffer.lines.len() {
                if active {
                    execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                } else {
                    execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                }
                print!("{:>width$} ", line_idx + 1, width = line_num_width);
                
                // Render line content
                if let Some(line) = self.buffer.lines.get(line_idx) {
                    if active {
                        execute!(io::stdout(), SetForegroundColor(Color::White))?;
                    } else {
                        execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                    }
                    
                    let display_line = if line.len() > content_width {
                        &line[..content_width]
                    } else {
                        line
                    };
                    print!("{}", display_line);
                    
                    // Clear the rest of the line to avoid artifacts
                    let used_width = line_num_width + 1 + display_line.len();
                    if used_width < width {
                        print!("{}", " ".repeat(width - used_width));
                    }
                }
            } else {
                // Empty line - just show line number area and clear the rest
                execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                print!("{}", " ".repeat(line_num_width + 1));
                print!("{}", " ".repeat(width.saturating_sub(line_num_width + 1)));
            }
            
            execute!(io::stdout(), ResetColor)?;
        }
        
        Ok(())
    }

    fn render_response_pane(&self, response: &ResponseBuffer, height: usize, width: usize, start_row: usize) -> Result<()> {
        let active = self.current_pane == Pane::Response;
        
        // Calculate line number width based on total lines
        let max_line_num = response.lines.len();
        let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
        let content_width = width.saturating_sub(line_num_width + 1); // +1 for space separator
        
        for i in 0..height {
            execute!(io::stdout(), cursor::MoveTo(0, (start_row + i) as u16))?;
            
            let line_idx = i + response.scroll_offset;
            
            // Render line number
            if line_idx < response.lines.len() {
                if active {
                    execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                } else {
                    execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                }
                print!("{:>width$} ", line_idx + 1, width = line_num_width);
                
                // Render line content
                if let Some(line) = response.lines.get(line_idx) {
                    if active {
                        execute!(io::stdout(), SetForegroundColor(Color::White))?;
                    } else {
                        execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                    }
                    
                    let display_line = if line.len() > content_width {
                        &line[..content_width]
                    } else {
                        line
                    };
                    print!("{}", display_line);
                    
                    // Clear the rest of the line to avoid artifacts
                    let used_width = line_num_width + 1 + display_line.len();
                    if used_width < width {
                        print!("{}", " ".repeat(width - used_width));
                    }
                }
            } else {
                // Empty line - just show line number area and clear the rest
                execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                print!("{}", " ".repeat(line_num_width + 1));
                print!("{}", " ".repeat(width.saturating_sub(line_num_width + 1)));
            }
            
            execute!(io::stdout(), ResetColor)?;
        }
        
        Ok(())
    }

    fn render_status_line(&self, row: usize, width: usize) -> Result<()> {
        execute!(io::stdout(), cursor::MoveTo(0, row as u16))?;
        execute!(io::stdout(), SetBackgroundColor(Color::DarkBlue))?;
        execute!(io::stdout(), SetForegroundColor(Color::White))?;
        
        let mode_str = match self.mode {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Command => "COMMAND",
            EditorMode::Visual => "VISUAL",
            EditorMode::VisualLine => "VISUAL LINE",
        };
        
        let status = if let Some(ref response_status) = self.last_response_status {
            format!(" {} | {} | {} | {}", mode_str, self.current_pane_name(), response_status, self.status_message)
        } else {
            format!(" {} | {} | {}", mode_str, self.current_pane_name(), self.status_message)
        };
        
        let padded_status = if status.len() > width {
            status[..width].to_string()
        } else {
            format!("{}{}", status, " ".repeat(width - status.len()))
        };
        
        print!("{}", padded_status);
        execute!(io::stdout(), ResetColor)?;
        
        Ok(())
    }

    fn current_pane_name(&self) -> &str {
        match self.current_pane {
            Pane::Request => "Request",
            Pane::Response => "Response",
        }
    }

    fn get_response_pane_height(&self) -> usize {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let input_height = (total_content_height as f64 * self.pane_split_ratio) as usize;
        total_content_height - input_height
    }

    fn get_request_pane_height(&self) -> usize {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        (total_content_height as f64 * self.pane_split_ratio) as usize
    }

    fn expand_input_pane(&mut self) {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let min_output_height = 3;
        let max_input_height = total_content_height - min_output_height;
        let current_input_height = self.get_request_pane_height();
        
        if current_input_height < max_input_height {
            let new_input_height = (current_input_height + 1).min(max_input_height);
            self.pane_split_ratio = new_input_height as f64 / total_content_height as f64;
            self.status_message = format!("Input pane expanded to {} lines", new_input_height);
        }
    }

    fn expand_output_pane(&mut self) {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let min_input_height = 3;
        let max_output_height = total_content_height - min_input_height;
        let current_output_height = self.get_response_pane_height();
        
        if current_output_height < max_output_height {
            let new_output_height = (current_output_height + 1).min(max_output_height);
            let new_input_height = total_content_height - new_output_height;
            self.pane_split_ratio = new_input_height as f64 / total_content_height as f64;
            self.status_message = format!("Output pane expanded to {} lines", new_output_height);
        }
    }

    fn shrink_input_pane(&mut self) {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let min_input_height = 3;
        let current_input_height = self.get_request_pane_height();
        
        if current_input_height > min_input_height {
            let new_input_height = (current_input_height - 1).max(min_input_height);
            self.pane_split_ratio = new_input_height as f64 / total_content_height as f64;
            self.status_message = format!("Input pane shrunk to {} lines", new_input_height);
        }
    }

    fn shrink_output_pane(&mut self) {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let min_output_height = 3;
        let current_output_height = self.get_response_pane_height();
        
        if current_output_height > min_output_height {
            let new_output_height = (current_output_height - 1).max(min_output_height);
            let new_input_height = total_content_height - new_output_height;
            self.pane_split_ratio = new_input_height as f64 / total_content_height as f64;
            self.status_message = format!("Output pane shrunk to {} lines", new_output_height);
        }
    }

    fn maximize_input_pane(&mut self) {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let min_output_height = 3;
        let max_input_height = total_content_height - min_output_height;
        
        self.pane_split_ratio = max_input_height as f64 / total_content_height as f64;
        self.status_message = format!("Input pane maximized to {} lines", max_input_height);
    }

    fn maximize_output_pane(&mut self) {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let min_input_height = 3;
        let max_output_height = total_content_height - min_input_height;
        
        self.pane_split_ratio = min_input_height as f64 / total_content_height as f64;
        self.status_message = format!("Output pane maximized to {} lines", max_output_height);
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
