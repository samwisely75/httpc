//! # HTTPC REPL: Vim-Style HTTP Client Interface
//!
//! ## ARCHITECTURE OVERVIEW
//!
//! This module implements a dual-pane vim-style REPL for HTTP testing with
//! flicker elimination. The interface consists of:
//!
//! - **Request Pane** (top): Editable HTTP request content (method, URL, headers, body)
//! - **Response Pane** (bottom): Read-only HTTP response display (status, headers, body)
//! - **Status Line** (bottom): Mode indicator, command input, request timing
//!
//! ## FLICKER ELIMINATION SYSTEM
//!
//! **PROBLEM**: Traditional terminal applications redraw the entire screen for every
//! change, causing visual flicker that degrades user experience.
//!
//! **SOLUTION**: Three-tier rendering system that uses the minimal
//! update strategy for each type of user interaction:
//!
//! 1. **Pure Navigation** → `render()` → Direct cursor positioning (~0.1ms)
//! 2. **Content Editing** → `render_pane_update()` → Single pane update (~1-2ms)  
//! 3. **UI State Changes** → `render_full()` → Complete redraw (~2-5ms)
//!
//! **KEY INSIGHT**: When typing in the Request pane, the Response pane is static
//! and doesn't need redrawing. By isolating updates to only the changed pane,
//! we eliminate cross-pane flicker while maintaining responsiveness.
//!
//! ## TECHNICAL APPROACH
//!
//! - **Buffer-based rendering**: Collect all output in memory, write atomically
//! - **Minimal ANSI sequences**: Direct cursor positioning for navigation
//! - **Categorized render decisions**: Classify every key press into appropriate tier
//! - **Single-flush strategy**: One stdout.flush() per update cycle
//!
//! This results in a flicker-free vim-style interface that feels responsive
//! and professional, suitable for intensive HTTP development workflows.

use std::collections::HashMap;
use std::io::{self, Write};
use std::time::Instant;

use anyhow::Result;
use crossterm::{
    cursor::{self, SetCursorStyle},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
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
    request_start_time: Option<Instant>,
    last_request_duration: Option<u64>, // in milliseconds
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
        self.scroll_offset = 0; // Ensure we scroll to top
    }

    fn move_cursor_to_end(&mut self) {
        if !self.lines.is_empty() {
            self.cursor_line = self.lines.len() - 1;
            self.cursor_col = self.lines[self.cursor_line].len();
        }
    }

    fn move_cursor_to_end_with_scroll(&mut self, visible_height: usize) {
        if !self.lines.is_empty() {
            self.cursor_line = self.lines.len() - 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            
            // Adjust scroll to show the end of the buffer
            if self.lines.len() > visible_height {
                self.scroll_offset = self.lines.len() - visible_height;
            } else {
                self.scroll_offset = 0;
            }
        }
    }

    fn scroll_half_page_up_with_cursor(&mut self, half_page_size: usize) {
        let scroll_amount = half_page_size.min(self.scroll_offset);
        self.scroll_offset -= scroll_amount;
        
        // Move cursor to maintain relative position or to top of visible area
        if self.cursor_line >= scroll_amount {
            self.cursor_line -= scroll_amount;
        } else {
            self.cursor_line = self.scroll_offset;
        }
        
        // Ensure cursor column is within line bounds
        let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
        self.cursor_col = self.cursor_col.min(line_len);
    }

    fn scroll_half_page_down_with_cursor(&mut self, half_page_size: usize, visible_height: usize) {
        let max_scroll = if self.lines.len() > visible_height {
            self.lines.len() - visible_height
        } else {
            0
        };
        let scroll_amount = half_page_size.min(max_scroll - self.scroll_offset);
        self.scroll_offset += scroll_amount;
        
        // Move cursor to maintain relative position or to bottom of visible area
        let new_cursor_line = self.cursor_line + scroll_amount;
        if new_cursor_line < self.lines.len() {
            self.cursor_line = new_cursor_line;
        } else {
            self.cursor_line = self.lines.len().saturating_sub(1);
        }
        
        // Ensure cursor column is within line bounds
        let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
        self.cursor_col = self.cursor_col.min(line_len);
    }

    fn move_cursor_word_forward(&mut self) {
        loop {
            if let Some(line) = self.lines.get(self.cursor_line) {
                let chars: Vec<char> = line.chars().collect();
                let mut pos = self.cursor_col;
                
                if pos >= chars.len() {
                    // At end of line, move to next line
                    if self.cursor_line < self.lines.len() - 1 {
                        self.cursor_line += 1;
                        self.cursor_col = 0;
                        // Skip leading whitespace on new line
                        if let Some(new_line) = self.lines.get(self.cursor_line) {
                            let new_chars: Vec<char> = new_line.chars().collect();
                            while self.cursor_col < new_chars.len() && new_chars[self.cursor_col].is_whitespace() {
                                self.cursor_col += 1;
                            }
                        }
                        return;
                    } else {
                        return; // Already at end of buffer
                    }
                }
                
                let start_char = chars[pos];
                
                if start_char.is_whitespace() {
                    // Skip whitespace
                    while pos < chars.len() && chars[pos].is_whitespace() {
                        pos += 1;
                    }
                } else if start_char.is_alphanumeric() || start_char == '_' {
                    // Skip alphanumeric word
                    while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                        pos += 1;
                    }
                } else {
                    // Skip punctuation/symbols (treat each as separate word)
                    while pos < chars.len() && !chars[pos].is_alphanumeric() && !chars[pos].is_whitespace() && chars[pos] != '_' {
                        pos += 1;
                    }
                }
                
                if pos < chars.len() {
                    self.cursor_col = pos;
                    return;
                } else {
                    // Reached end of line, try next line
                    if self.cursor_line < self.lines.len() - 1 {
                        self.cursor_line += 1;
                        self.cursor_col = 0;
                        continue; // Loop to handle new line
                    } else {
                        self.cursor_col = chars.len();
                        return;
                    }
                }
            } else {
                return;
            }
        }
    }

    fn move_cursor_word_backward(&mut self) {
        loop {
            if let Some(line) = self.lines.get(self.cursor_line) {
                let chars: Vec<char> = line.chars().collect();
                let mut pos = self.cursor_col;
                
                if pos == 0 {
                    // At beginning of line, move to previous line
                    if self.cursor_line > 0 {
                        self.cursor_line -= 1;
                        if let Some(prev_line) = self.lines.get(self.cursor_line) {
                            self.cursor_col = prev_line.len();
                            continue; // Loop to handle previous line
                        }
                    }
                    return; // Already at beginning of buffer
                }
                
                pos -= 1;
                
                // Skip trailing whitespace if we're starting on whitespace
                if pos < chars.len() && chars[pos].is_whitespace() {
                    while pos > 0 && chars[pos].is_whitespace() {
                        pos -= 1;
                    }
                    if pos == 0 && chars[pos].is_whitespace() {
                        self.cursor_col = 0;
                        return;
                    }
                }
                
                if pos < chars.len() {
                    let current_char = chars[pos];
                    
                    if current_char.is_alphanumeric() || current_char == '_' {
                        // Move to beginning of alphanumeric word
                        while pos > 0 && (chars[pos - 1].is_alphanumeric() || chars[pos - 1] == '_') {
                            pos -= 1;
                        }
                    } else if !current_char.is_whitespace() {
                        // Move to beginning of punctuation sequence
                        while pos > 0 && !chars[pos - 1].is_alphanumeric() && !chars[pos - 1].is_whitespace() && chars[pos - 1] != '_' {
                            pos -= 1;
                        }
                    }
                }
                
                self.cursor_col = pos;
                return;
            } else {
                return;
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

    fn move_cursor_down_with_scroll(&mut self, visible_height: usize) {
        if self.cursor_line < self.lines.len().saturating_sub(1) {
            self.cursor_line += 1;
            let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
            self.cursor_col = self.cursor_col.min(line_len);
            
            // Auto-scroll down if cursor goes below visible area
            if self.cursor_line >= self.scroll_offset + visible_height {
                self.scroll_offset = self.cursor_line - visible_height + 1;
            }
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

    fn move_cursor_word_forward(&mut self) {
        loop {
            if let Some(line) = self.lines.get(self.cursor_line) {
                let chars: Vec<char> = line.chars().collect();
                let mut pos = self.cursor_col;
                
                if pos >= chars.len() {
                    // At end of line, move to next line
                    if self.cursor_line < self.lines.len() - 1 {
                        self.cursor_line += 1;
                        self.cursor_col = 0;
                        // Skip leading whitespace on new line
                        if let Some(new_line) = self.lines.get(self.cursor_line) {
                            let new_chars: Vec<char> = new_line.chars().collect();
                            while self.cursor_col < new_chars.len() && new_chars[self.cursor_col].is_whitespace() {
                                self.cursor_col += 1;
                            }
                        }
                        return;
                    } else {
                        return; // Already at end of buffer
                    }
                }
                
                let start_char = chars[pos];
                
                if start_char.is_whitespace() {
                    // Skip whitespace
                    while pos < chars.len() && chars[pos].is_whitespace() {
                        pos += 1;
                    }
                } else if start_char.is_alphanumeric() || start_char == '_' {
                    // Skip alphanumeric word
                    while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                        pos += 1;
                    }
                } else {
                    // Skip punctuation/symbols (treat each as separate word)
                    while pos < chars.len() && !chars[pos].is_alphanumeric() && !chars[pos].is_whitespace() && chars[pos] != '_' {
                        pos += 1;
                    }
                }
                
                if pos < chars.len() {
                    self.cursor_col = pos;
                    return;
                } else {
                    // Reached end of line, try next line
                    if self.cursor_line < self.lines.len() - 1 {
                        self.cursor_line += 1;
                        self.cursor_col = 0;
                        continue; // Loop to handle new line
                    } else {
                        self.cursor_col = chars.len();
                        return;
                    }
                }
            } else {
                return;
            }
        }
    }

    fn move_cursor_word_backward(&mut self) {
        loop {
            if let Some(line) = self.lines.get(self.cursor_line) {
                let chars: Vec<char> = line.chars().collect();
                let mut pos = self.cursor_col;
                
                if pos == 0 {
                    // At beginning of line, move to previous line
                    if self.cursor_line > 0 {
                        self.cursor_line -= 1;
                        if let Some(prev_line) = self.lines.get(self.cursor_line) {
                            self.cursor_col = prev_line.len();
                            continue; // Loop to handle previous line
                        }
                    }
                    return; // Already at beginning of buffer
                }
                
                pos -= 1;
                
                // Skip trailing whitespace if we're starting on whitespace
                if pos < chars.len() && chars[pos].is_whitespace() {
                    while pos > 0 && chars[pos].is_whitespace() {
                        pos -= 1;
                    }
                    if pos == 0 && chars[pos].is_whitespace() {
                        self.cursor_col = 0;
                        return;
                    }
                }
                
                if pos < chars.len() {
                    let current_char = chars[pos];
                    
                    if current_char.is_alphanumeric() || current_char == '_' {
                        // Move to beginning of alphanumeric word
                        while pos > 0 && (chars[pos - 1].is_alphanumeric() || chars[pos - 1] == '_') {
                            pos -= 1;
                        }
                    } else if !current_char.is_whitespace() {
                        // Move to beginning of punctuation sequence
                        while pos > 0 && !chars[pos - 1].is_alphanumeric() && !chars[pos - 1].is_whitespace() && chars[pos - 1] != '_' {
                            pos -= 1;
                        }
                    }
                }
                
                self.cursor_col = pos;
                return;
            } else {
                return;
            }
        }
    }

    fn move_cursor_to_start(&mut self) {
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0; // Ensure we scroll to top
    }

    fn move_cursor_to_end(&mut self) {
        if !self.lines.is_empty() {
            self.cursor_line = self.lines.len() - 1;
            self.cursor_col = self.lines[self.cursor_line].len();
        }
    }

    fn move_cursor_to_end_with_scroll(&mut self, visible_height: usize) {
        if !self.lines.is_empty() {
            self.cursor_line = self.lines.len() - 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            
            // Adjust scroll to show the end of the buffer
            if self.lines.len() > visible_height {
                self.scroll_offset = self.lines.len() - visible_height;
            } else {
                self.scroll_offset = 0;
            }
        }
    }

    fn scroll_half_page_up_with_cursor(&mut self, half_page_size: usize) {
        let scroll_amount = half_page_size.min(self.scroll_offset);
        self.scroll_offset -= scroll_amount;
        
        // Move cursor to maintain relative position or to top of visible area
        if self.cursor_line >= scroll_amount {
            self.cursor_line -= scroll_amount;
        } else {
            self.cursor_line = self.scroll_offset;
        }
        
        // Ensure cursor column is within line bounds
        let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
        self.cursor_col = self.cursor_col.min(line_len);
    }

    fn scroll_half_page_down_with_cursor(&mut self, half_page_size: usize, visible_height: usize) {
        let max_scroll = if self.lines.len() > visible_height {
            self.lines.len() - visible_height
        } else {
            0
        };
        let scroll_amount = half_page_size.min(max_scroll - self.scroll_offset);
        self.scroll_offset += scroll_amount;
        
        // Move cursor to maintain relative position or to bottom of visible area
        let new_cursor_line = self.cursor_line + scroll_amount;
        if new_cursor_line < self.lines.len() {
            self.cursor_line = new_cursor_line;
        } else {
            self.cursor_line = self.lines.len().saturating_sub(1);
        }
        
        // Ensure cursor column is within line bounds
        let line_len = self.lines.get(self.cursor_line).map_or(0, |l| l.len());
        self.cursor_col = self.cursor_col.min(line_len);
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
            request_start_time: None,
            last_request_duration: None,
            pane_split_ratio: 0.5, // Start with 50/50 split
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Enable raw mode and alternate screen for proper terminal isolation
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        
        // Clear screen once at startup and move cursor to top
        execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        
        // Set initial cursor style for insert mode
        execute!(io::stdout(), SetCursorStyle::BlinkingBar)?;
        
        let result = self.event_loop().await;
        
        // Clean up - restore default cursor and exit alternate screen
        execute!(io::stdout(), SetCursorStyle::DefaultUserShape)?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        
        result
    }

    /// Main event processing loop with rendering optimization.
    /// 
    /// Eliminate terminal flicker by using the minimal rendering strategy
    /// for each type of user interaction. Terminal flicker occurs when we redraw
    /// content unnecessarily, causing visual artifacts and poor user experience.
    /// 
    /// Traditional terminal applications often use a single "redraw everything" 
    /// approach, but for a vim-style dual-pane interface, this causes severe flicker. 
    /// We implement a three-tier rendering system:
    /// 
    /// 1. Pure cursor movement (render) - Just position cursor, no screen writes
    /// 2. Content updates (render_pane_update) - Update only the changed pane
    /// 3. Full updates (render_full) - Redraw everything (used sparingly)
    /// 
    /// This approach eliminates flicker while maintaining responsiveness.
    async fn event_loop(&mut self) -> Result<()> {
        // Initial full render
        self.render_full()?;
        
        loop {
            match event::read()? {
                Event::Key(key) => {
                    let old_mode = self.mode.clone();
                    let old_pane = self.current_pane.clone();
                    
                    // Capture scroll state before handling the key event
                    // Detect when movement commands cause scrolling because movement commands 
                    // (hjkl, arrow keys) can trigger scrolling when the cursor moves beyond the 
                    // visible area. Without this detection, these would be treated as "pure cursor 
                    // movement" which would cause content to not refresh properly when scrolling occurred.
                    let old_request_scroll = self.buffer.scroll_offset;
                    let old_response_scroll = self.response_buffer.as_ref().map(|r| r.scroll_offset);
                    
                    // Capture pane split ratio before handling the key event
                    // Detect when pane boundary control commands (Ctrl+J/Ctrl+K) change layout
                    // because pane boundary changes affect both panes' dimensions and require full 
                    // redraw, but without this the rendering decision logic wouldn't detect these 
                    // layout changes.
                    let old_pane_split_ratio = self.pane_split_ratio;
                    
                    let needs_exit = self.handle_key_event(key).await?;
                    
                    if needs_exit {
                        break;
                    }
                    
                    // Check if scrolling occurred during the key event
                    // When movement commands (hjkl, arrows) cause the cursor to move
                    // beyond the visible area, the scroll-aware movement methods automatically
                    // adjust the scroll_offset. We detect this change to trigger proper rendering.
                    let scroll_occurred = self.buffer.scroll_offset != old_request_scroll ||
                        self.response_buffer.as_ref().map(|r| r.scroll_offset) != old_response_scroll;
                    
                    // Check if pane layout changed during the key event  
                    // Pane boundary controls (Ctrl+J/Ctrl+K) modify the pane_split_ratio,
                    // changing both panes' dimensions. This requires a full redraw since both
                    // panes' content positioning and the separator location change.
                    let pane_layout_changed = self.pane_split_ratio != old_pane_split_ratio;
                    
                    // RENDERING DECISION LOGIC
                    // ======================
                    // We categorize every possible user action into one of three
                    // rendering strategies to minimize flicker and maximize performance.
                    
                    // TIER 3: Full screen redraw (most expensive, used sparingly)
                    // When: UI state changes that affect layout or multiple panes
                    let needs_full_render = 
                        self.mode != old_mode ||                    // Mode changed (affects status, cursor style, highlighting)
                        self.current_pane != old_pane ||            // Pane switched (affects focus indicators, cursor position)
                        self.mode == EditorMode::Command ||         // Command mode (bottom status line changes constantly)
                        self.mode == EditorMode::Visual ||          // Visual mode (requires selection highlighting)
                        self.mode == EditorMode::VisualLine ||      // Visual line mode (requires line highlighting)
                        scroll_occurred ||                          // Movement caused scrolling (content repositions)
                        pane_layout_changed ||                      // Pane boundary changed (layout repositions)
                        (key.modifiers.contains(KeyModifiers::CONTROL) && 
                         matches!(key.code, KeyCode::Char('u') | KeyCode::Char('d') | KeyCode::Char('f') | KeyCode::Char('b') | KeyCode::Char('g'))) || // Scrolling commands (content repositions)
                        matches!(key.code, KeyCode::PageUp | KeyCode::PageDown); // Page scrolling (major content changes)
                    
                    // TIER 2: Single pane content update (moderate cost, efficient)
                    // When: Content changes within one pane, other pane stays untouched
                    let needs_content_update = 
                        matches!(key.code, KeyCode::Enter) ||       // New line added (content area expands)
                        matches!(key.code, KeyCode::Delete | KeyCode::Backspace) || // Content deleted (text removal)
                        (self.mode == EditorMode::Insert && matches!(key.code, KeyCode::Char(_)) && !key.modifiers.contains(KeyModifiers::CONTROL)); // Regular typing (character insertion)
                    
                    // TIER 1: Pure cursor movement (fastest, zero flicker)
                    // When: Only cursor position changes, no content or UI state changes
                    // Examples: hjkl keys, arrow keys, word movement (w/b), line start/end (0/$)
                    
                    // Apply the appropriate rendering strategy
                    if needs_full_render {
                        self.render_full()?;
                    } else if needs_content_update {
                        // Content changed - update only the active pane efficiently
                        // This prevents the Response pane from flickering when typing in Request pane
                        self.render_pane_update()?;
                    } else {
                        // Pure cursor movement - fastest possible update
                        // Uses direct ANSI escape sequences for cursor positioning only
                        self.render()?;
                    }
                }
                Event::Resize(width, height) => {
                    self.terminal_size = (width, height);
                    // Full render needed on resize - layout completely changes
                    self.render_full()?;
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
            KeyCode::Char('w') => {
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
                    self.pending_ctrl_w = false;
                    return Ok(false);
                }
                KeyCode::Esc => {
                    // Cancel Ctrl+W command
                    self.pending_ctrl_w = false;
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
                        self.buffer.scroll_half_page_up_with_cursor(half_page);
                    } else if self.response_buffer.is_some() {
                        let half_page = self.get_response_pane_height() / 2;
                        if let Some(ref mut response) = self.response_buffer {
                            response.scroll_half_page_up_with_cursor(half_page);
                        }
                    }
                    return Ok(false);
                }
                KeyCode::Char('d') => {
                    if self.current_pane == Pane::Request {
                        let half_page = self.get_request_pane_height() / 2;
                        let visible_height = self.get_request_pane_height();
                        self.buffer.scroll_half_page_down_with_cursor(half_page, visible_height);
                    } else if self.response_buffer.is_some() {
                        let half_page = self.get_response_pane_height() / 2;
                        let visible_height = self.get_response_pane_height();
                        if let Some(ref mut response) = self.response_buffer {
                            response.scroll_half_page_down_with_cursor(half_page, visible_height);
                        }
                    }
                    return Ok(false);
                }
                KeyCode::Char('f') => {
                    // Ctrl+F - Scroll forward (down) one full page
                    if self.current_pane == Pane::Request {
                        let full_page = self.get_request_pane_height();
                        let visible_height = self.get_request_pane_height();
                        self.buffer.scroll_half_page_down_with_cursor(full_page, visible_height);
                    } else if self.response_buffer.is_some() {
                        let full_page = self.get_response_pane_height();
                        let visible_height = self.get_response_pane_height();
                        if let Some(ref mut response) = self.response_buffer {
                            response.scroll_half_page_down_with_cursor(full_page, visible_height);
                        }
                    }
                    return Ok(false);
                }
                KeyCode::Char('b') => {
                    // Ctrl+B - Scroll backward (up) one full page
                    if self.current_pane == Pane::Request {
                        let full_page = self.get_request_pane_height();
                        self.buffer.scroll_half_page_up_with_cursor(full_page);
                    } else if self.response_buffer.is_some() {
                        let full_page = self.get_response_pane_height();
                        if let Some(ref mut response) = self.response_buffer {
                            response.scroll_half_page_up_with_cursor(full_page);
                        }
                    }
                    return Ok(false);
                }
                KeyCode::Char('k') => {
                    // Ctrl+K - Boundary control (downward)
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
                KeyCode::Char('j') => {
                    // Ctrl+J - Boundary control (upward)
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
                }
            }
            KeyCode::Char('I') => {
                // Insert at beginning of line - only in request pane
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_line_start();
                    self.mode = EditorMode::Insert;
                    self.status_message = "-- INSERT --".to_string();
                    execute!(io::stdout(), SetCursorStyle::BlinkingBar)?;
                }
            }
            KeyCode::Char('A') => {
                // Append at end of line - only in request pane
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_line_end();
                    self.mode = EditorMode::Insert;
                    self.status_message = "-- INSERT --".to_string();
                    execute!(io::stdout(), SetCursorStyle::BlinkingBar)?;
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
                }
            }
            KeyCode::Char('U') => {
                // Shift+U - scroll up half page (vim-style)
                if self.current_pane == Pane::Request {
                    let half_page = self.get_request_pane_height() / 2;
                    self.buffer.scroll_half_page_up(half_page);
                } else if self.response_buffer.is_some() {
                    let half_page = self.get_response_pane_height() / 2;
                    if let Some(ref mut response) = self.response_buffer {
                        for _ in 0..half_page {
                            response.scroll_up();
                        }
                    }
                }
            }
            KeyCode::Char('D') => {
                // Shift+D in normal mode - scroll down half page (vim-style)
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    if self.current_pane == Pane::Request {
                        let half_page = self.get_request_pane_height() / 2;
                        let max_lines = self.get_request_pane_height();
                        self.buffer.scroll_half_page_down(half_page, max_lines);
                    } else if self.response_buffer.is_some() {
                        let half_page = self.get_response_pane_height() / 2;
                        let max_lines = self.get_response_pane_height();
                        if let Some(ref mut response) = self.response_buffer {
                            for _ in 0..half_page {
                                response.scroll_down(max_lines);
                            }
                        }
                    }
                } else {
                    // Regular D - Delete from cursor to end of line
                    if self.current_pane == Pane::Request {
                        let deleted = self.buffer.delete_to_line_end();
                        if !deleted.is_empty() {
                            self.clipboard = deleted;
                        }
                    }
                }
            }
            KeyCode::Char('b') => {
                // Move cursor backward by word
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_word_backward();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_word_backward();
                }
            }
            KeyCode::Char('w') => {
                // Move cursor forward by word
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_word_forward();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_word_forward();
                }
            }
            KeyCode::Char('g') => {
                // Handle 'gg' command
                if self.current_pane == Pane::Request {
                    if self.pending_g {
                        // Second 'g' - go to start of buffer
                        self.buffer.move_cursor_to_start();
                        self.pending_g = false;
                    } else {
                        // First 'g' - wait for second 'g'
                        self.pending_g = true;
                    }
                } else if let Some(ref mut response) = self.response_buffer {
                    if self.pending_g {
                        // Second 'g' - go to start of buffer
                        response.move_cursor_to_start();
                        self.pending_g = false;
                    } else {
                        // First 'g' - wait for second 'g'
                        self.pending_g = true;
                    }
                }
            }
            KeyCode::Char('G') => {
                // Go to end of buffer
                if self.current_pane == Pane::Request {
                    let visible_height = self.get_request_pane_height();
                    self.buffer.move_cursor_to_end_with_scroll(visible_height);
                } else if self.response_buffer.is_some() {
                    let visible_height = self.get_response_pane_height();
                    if let Some(ref mut response) = self.response_buffer {
                        response.move_cursor_to_end_with_scroll(visible_height);
                    }
                }
            }
            KeyCode::Char('0') => {
                // Move to beginning of line
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_line_start();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_to_line_start();
                }
            }
            KeyCode::Char('$') => {
                // Move to end of line
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_line_end();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_to_line_end();
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
                } else if self.current_pane == Pane::Response && self.response_buffer.is_some() {
                    self.mode = EditorMode::Visual;
                    if let Some(ref response) = self.response_buffer {
                        self.visual_selection = Some(VisualSelection {
                            start_line: response.cursor_line,
                            start_col: response.cursor_col,
                            end_line: response.cursor_line,
                            end_col: response.cursor_col,
                        });
                    }
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
                } else if self.current_pane == Pane::Response && self.response_buffer.is_some() {
                    self.mode = EditorMode::VisualLine;
                    if let Some(ref response) = self.response_buffer {
                        self.visual_selection = Some(VisualSelection {
                            start_line: response.cursor_line,
                            start_col: 0,
                            end_line: response.cursor_line,
                            end_col: response.lines.get(response.cursor_line).map_or(0, |l| l.len()),
                        });
                    }
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
                    let visible_height = self.get_request_pane_height();
                    self.buffer.move_cursor_down_with_scroll(visible_height);
                } else if self.response_buffer.is_some() {
                    let visible_height = self.get_response_pane_height();
                    if let Some(ref mut response) = self.response_buffer {
                        response.move_cursor_down_with_scroll(visible_height);
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_up_with_scroll();
                } else if self.response_buffer.is_some() {
                    if let Some(ref mut response) = self.response_buffer {
                        response.move_cursor_up_with_scroll();
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
            KeyCode::Char('y') => {
                // Start yank operation - for now just yank current line
                if self.current_pane == Pane::Request {
                    self.clipboard = self.buffer.yank_line();
                } else if let Some(ref response) = self.response_buffer {
                    let line_idx = response.scroll_offset;
                    if let Some(line) = response.lines.get(line_idx) {
                        self.clipboard = line.clone();
                    }
                }
            }
            KeyCode::Char('Y') => {
                // Yank from cursor to end of line
                if self.current_pane == Pane::Request {
                    self.clipboard = self.buffer.yank_to_line_end();
                }
            }
            KeyCode::Char('p') => {
                if self.current_pane == Pane::Request && !self.clipboard.is_empty() {
                    self.buffer.paste_line(&self.clipboard);
                }
            }
            KeyCode::Char('P') => {
                // Paste above current line
                if self.current_pane == Pane::Request && !self.clipboard.is_empty() {
                    self.buffer.paste_line_above(&self.clipboard);
                }
            }
            KeyCode::Char('x') => {
                if self.current_pane == Pane::Request {
                    self.buffer.delete_char_at_cursor();
                }
            }
            KeyCode::Delete => {
                if self.current_pane == Pane::Request {
                    self.buffer.delete_char_at_cursor();
                }
            }
            KeyCode::Esc => {
                self.status_message = "".to_string();
            }
            KeyCode::PageUp => {
                if self.current_pane == Pane::Request {
                    let half_page = self.get_request_pane_height() / 2;
                    self.buffer.scroll_half_page_up(half_page);
                } else if self.response_buffer.is_some() {
                    let half_page = self.get_response_pane_height() / 2;
                    if let Some(ref mut response) = self.response_buffer {
                        response.scroll_half_page_up(half_page);
                    }
                }
            }
            KeyCode::PageDown => {
                if self.current_pane == Pane::Request {
                    let half_page = self.get_request_pane_height() / 2;
                    let max_lines = self.get_request_pane_height();
                    self.buffer.scroll_half_page_down(half_page, max_lines);
                } else if self.response_buffer.is_some() {
                    let half_page = self.get_response_pane_height() / 2;
                    let max_lines = self.get_response_pane_height();
                    if let Some(ref mut response) = self.response_buffer {
                        response.scroll_half_page_down(half_page, max_lines);
                    }
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
                let visible_height = self.get_request_pane_height();
                self.buffer.new_line_with_scroll(visible_height);
            }
            KeyCode::Char(ch) => {
                // Handle Ctrl+character combinations
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match ch {
                        'm' => {
                            // Ctrl+M - Maximize current pane
                            if self.current_pane == Pane::Request {
                                if self.response_buffer.is_some() {
                                    // Maximize input pane
                                    self.maximize_input_pane();
                                } else {
                                    // Do nothing if no response buffer
                                }
                            }
                        }
                        'j' => {
                            // Ctrl+J - Boundary control (upward) or execute request
                            if self.current_pane == Pane::Request {
                                if self.response_buffer.is_some() {
                                    // Shrink input pane
                                    self.shrink_input_pane();
                                } else {
                                    // Execute request if no response buffer
                                    self.execute_request().await?
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
                            // Ctrl+K - Boundary control (downward) in insert mode
                            if self.current_pane == Pane::Request && self.response_buffer.is_some() {
                                // Expand input pane
                                self.expand_input_pane();
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
                self.status_message = "".to_string(); // Clear status message after command execution
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
                        if self.current_pane == Pane::Request {
                            sel.end_col = self.buffer.lines.get(self.buffer.cursor_line).map_or(0, |l| l.len());
                        } else if let Some(ref response) = self.response_buffer {
                            sel.end_col = response.lines.get(response.cursor_line).map_or(0, |l| l.len());
                        }
                    }
                } else {
                    self.mode = EditorMode::Visual;
                    self.status_message = "-- VISUAL --".to_string();
                    if let Some(ref mut sel) = self.visual_selection {
                        if self.current_pane == Pane::Request {
                            sel.end_col = self.buffer.cursor_col;
                        } else if let Some(ref response) = self.response_buffer {
                            sel.end_col = response.cursor_col;
                        }
                    }
                }
            }
            KeyCode::Char('y') => {
                // Yank selection
                if let Some(ref selection) = self.visual_selection {
                    let yanked_text = self.get_selected_text(selection);
                    self.clipboard = yanked_text;
                }
                self.mode = EditorMode::Normal;
                self.status_message = "".to_string();
                self.visual_selection = None;
            }
            KeyCode::Char('d') => {
                // Delete selection (only works in Request pane)
                if self.current_pane == Pane::Request {
                    if let Some(selection) = self.visual_selection.take() {
                        let deleted_text = self.delete_selected_text(&selection);
                        self.clipboard = deleted_text;
                    }
                }
                self.mode = EditorMode::Normal;
                self.status_message = "".to_string();
                self.visual_selection = None;
            }
            // Movement commands that update selection
            KeyCode::Char('h') | KeyCode::Left => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_left();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_left();
                }
                self.update_visual_selection();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.current_pane == Pane::Request {
                    let visible_height = self.get_request_pane_height();
                    self.buffer.move_cursor_down_with_scroll(visible_height);
                } else if self.response_buffer.is_some() {
                    if let Some(ref mut response) = self.response_buffer {
                        response.move_cursor_down();
                    }
                }
                self.update_visual_selection();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_up_with_scroll();
                } else if self.response_buffer.is_some() {
                    if let Some(ref mut response) = self.response_buffer {
                        response.move_cursor_up();
                    }
                }
                self.update_visual_selection();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_right();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_right();
                }
                self.update_visual_selection();
            }
            KeyCode::Char('w') => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_word_forward();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_word_forward();
                }
                self.update_visual_selection();
            }
            KeyCode::Char('b') => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_word_backward();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_word_backward();
                }
                self.update_visual_selection();
            }
            KeyCode::Char('0') => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_line_start();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_to_line_start();
                }
                self.update_visual_selection();
            }
            KeyCode::Char('$') => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_line_end();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_to_line_end();
                }
                self.update_visual_selection();
            }
            KeyCode::Char('G') => {
                if self.current_pane == Pane::Request {
                    self.buffer.move_cursor_to_end();
                } else if let Some(ref mut response) = self.response_buffer {
                    response.move_cursor_to_end();
                }
                self.update_visual_selection();
            }
            _ => {}
        }
        Ok(false)
    }

    fn update_visual_selection(&mut self) {
        if let Some(ref mut selection) = self.visual_selection {
            if self.current_pane == Pane::Request {
                selection.end_line = self.buffer.cursor_line;
                if self.mode == EditorMode::VisualLine {
                    selection.end_col = self.buffer.lines.get(self.buffer.cursor_line).map_or(0, |l| l.len());
                } else {
                    selection.end_col = self.buffer.cursor_col;
                }
            } else if let Some(ref response) = self.response_buffer {
                selection.end_line = response.cursor_line;
                if self.mode == EditorMode::VisualLine {
                    selection.end_col = response.lines.get(response.cursor_line).map_or(0, |l| l.len());
                } else {
                    selection.end_col = response.cursor_col;
                }
            }
        }
    }

    fn get_selected_text(&self, selection: &VisualSelection) -> String {
        let start_line = selection.start_line.min(selection.end_line);
        let end_line = selection.start_line.max(selection.end_line);
        let start_col = if selection.start_line == start_line { selection.start_col } else { selection.end_col };
        let end_col = if selection.end_line == end_line { selection.end_col } else { selection.start_col };

        // Choose the appropriate lines based on current pane
        let lines = if self.current_pane == Pane::Request {
            &self.buffer.lines
        } else if let Some(ref response) = self.response_buffer {
            &response.lines
        } else {
            return String::new();
        };

        if start_line == end_line {
            // Single line selection
            if let Some(line) = lines.get(start_line) {
                let start_idx = start_col.min(end_col);
                let end_idx = start_col.max(end_col);
                return line[start_idx..end_idx.min(line.len())].to_string();
            }
        } else {
            // Multi-line selection
            let mut result = String::new();
            for (i, line) in lines.iter().enumerate().skip(start_line).take(end_line - start_line + 1) {
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
                } else {
                    return Ok(true);
                }
            }
            "q!" | "quit!" => {
                // Force quit - exit immediately without any checks
                return Ok(true);
            }
            "x" | "execute" => {
                // Clear any existing response pane first
                self.response_buffer = None;
                // Execute the request in the request pane
                if self.current_pane == Pane::Request || self.response_buffer.is_none() {
                    self.execute_request().await?;
                } 
            }
            "clear" => {
                self.response_buffer = None;
                self.status_message = "Response cleared".to_string();
            }
            "verbose" => {
                self.verbose = !self.verbose;
            }
            _ => {
                if self.command_buffer.starts_with("head ") {
                    let parts: Vec<&str> = self.command_buffer.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        let key = parts[1].to_string();
                        let value = parts[2].to_string();
                        self.session_headers.insert(key.clone(), value.clone());
                        self.status_message = format!("Header set: {} = {}", key, value);
                    }
                } else if self.command_buffer.starts_with("unhead ") {
                    let parts: Vec<&str> = self.command_buffer.splitn(2, ' ').collect();
                    if parts.len() >= 2 {
                        let key = parts[1].to_string();
                        if self.session_headers.remove(&key).is_some() {
                            self.status_message = format!("Header removed: {}", key);
                        } else {
                            self.status_message = format!("Header not found: {}", key);
                        }
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
        
        // Skip empty line after URL if it exists, then rest becomes the body
        let body_start_idx = if lines.len() > 1 && lines[1].trim().is_empty() { 2 } else { 1 };
        let body = if lines.len() > body_start_idx {
            Some(lines[body_start_idx..].join("\n"))
        } else {
            None
        };
        
        let cmd = ReplCommand {
            method,
            url,
            body,
            headers: self.session_headers.clone(),
        };
        
        // Start timing the request
        self.request_start_time = Some(Instant::now());
        
        match self.client.request(&cmd).await {
            Ok(response) => {
                // Calculate request duration
                if let Some(start_time) = self.request_start_time.take() {
                    self.last_request_duration = Some(start_time.elapsed().as_millis() as u64);
                }
                
                let status = response.status();
                let headers = response.headers();
                let body = response.body();
                
                // Store the response status for display in status bar
                self.last_response_status = Some(format!("HTTP {} {}", 
                    status.as_u16(), 
                    status.canonical_reason().unwrap_or("")
                ));
                
                let mut response_text = String::new();
                
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
            }
            Err(e) => {
                // Calculate request duration
                if let Some(start_time) = self.request_start_time.take() {
                    self.last_request_duration = Some(start_time.elapsed().as_millis() as u64);
                }
                
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

    fn is_position_selected(&self, line: usize, col: usize) -> bool {
        if let Some(ref selection) = self.visual_selection {
            let start_line = selection.start_line.min(selection.end_line);
            let end_line = selection.start_line.max(selection.end_line);
            let (start_col, end_col) = if selection.start_line == start_line {
                (selection.start_col, selection.end_col)
            } else {
                (selection.end_col, selection.start_col)
            };

            if self.mode == EditorMode::VisualLine {
                // In visual line mode, select entire lines
                line >= start_line && line <= end_line
            } else {
                // In visual mode, select character range
                if line < start_line || line > end_line {
                    false
                } else if start_line == end_line {
                    // Single line selection
                    col >= start_col.min(end_col) && col < start_col.max(end_col)
                } else {
                    // Multi-line selection
                    if line == start_line {
                        col >= start_col
                    } else if line == end_line {
                        col < end_col
                    } else {
                        true // Middle lines are fully selected
                    }
                }
            }
        } else {
            false
        }
    }

    /// TIER 1: Pure cursor movement rendering (fastest, zero flicker)
    /// 
    /// Position cursor with minimal terminal I/O for navigation commands.
    /// When users press hjkl, arrow keys, or movement commands like w/b/0/$, 
    /// only the cursor position changes - no content modification. We use a single 
    /// ANSI escape sequence to move the cursor directly. Without this approach, 
    /// we'd need screen clearing or content redrawing which causes flicker.
    /// 
    /// WHEN USED: Pure navigation in Normal mode (hjkl, arrow keys, w/b, 0/$, gg/G)
    /// PERFORMANCE: ~0.1ms - Single escape sequence, no buffering overhead
    /// FLICKER: None - No screen clearing or content redrawing
    fn render(&self) -> Result<()> {
        let request_height = if self.response_buffer.is_some() { 
            self.get_request_pane_height() 
        } else { 
            self.terminal_size.1 as usize - 1
        };
        
        // Calculate cursor position based on current pane
        if self.current_pane == Pane::Request {
            let cursor_y = self.buffer.cursor_line.saturating_sub(self.buffer.scroll_offset);
            if cursor_y < request_height {
                let max_line_num = self.buffer.lines.len();
                let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
                let cursor_x = line_num_width + 1 + self.buffer.cursor_col;
                
                // Single ANSI escape sequence - direct cursor positioning
                // Format: \x1b[row;colH (1-indexed)
                print!("\x1b[{};{}H", cursor_y + 1, cursor_x + 1);
            }
        } else if self.current_pane == Pane::Response && self.response_buffer.is_some() {
            if let Some(ref response) = self.response_buffer {
                let response_height = self.get_response_pane_height();
                let cursor_y = response.cursor_line.saturating_sub(response.scroll_offset);
                if cursor_y < response_height {
                    let max_line_num = response.lines.len();
                    let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
                    let cursor_x = line_num_width + 1 + response.cursor_col;
                    
                    // Position cursor in response pane (offset by request pane height + separator)
                    print!("\x1b[{};{}H", request_height + 1 + cursor_y + 1, cursor_x + 1);
                }
            }
        }
        
        // Single flush - minimal I/O overhead
        io::stdout().flush()?;
        Ok(())
    }

    /// TIER 3: Full screen rendering (most expensive, used sparingly)
    /// 
    /// Complete UI redraw when layout, modes, or multiple panes change.
    /// Some operations require redrawing the entire interface:
    /// - Mode changes (affects status line, cursor style, visual highlighting)
    /// - Pane switching (focus indicators, cursor visibility)
    /// - Scrolling operations (content repositions in visible area)
    /// - Visual mode (selection highlighting across content)
    /// 
    /// We use a single-buffer strategy: collect all output in memory, then
    /// write everything at once. Without this approach, partial content would
    /// be visible during the redraw which causes flicker.
    /// 
    /// WHEN USED: Mode changes, pane switching, scrolling, visual selections
    /// PERFORMANCE: ~2-5ms - Full UI redraw with buffering
    /// FLICKER: Minimal - Single atomic write prevents partial updates
    fn render_full(&self) -> Result<()> {
        // Collect all output in a single buffer to minimize terminal I/O
        let mut output_buffer = String::new();
        
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
        
        // Hide cursor during rendering to prevent cursor flicker
        output_buffer.push_str("\x1b[?25l");
        
        // Render request pane (top)
        self.render_request_pane_to_buffer(&mut output_buffer, request_height, width);
        
        // Render horizontal separator only if there's a response buffer
        if self.response_buffer.is_some() {
            output_buffer.push_str(&format!("\x1b[{};1H", request_height + 1)); // Move to separator row
            output_buffer.push_str("\x1b[36m"); // Cyan color
            output_buffer.push_str(&"─".repeat(width));
            output_buffer.push_str("\x1b[0m"); // Reset color
        }
        
        // Render response pane (bottom)
        if let Some(ref response) = self.response_buffer {
            self.render_response_pane_to_buffer(&mut output_buffer, response, response_height, width, request_height + 1);
        }
        
        // Render status line
        self.render_status_line_to_buffer(&mut output_buffer, height - 1, width);
        
        // Position cursor based on current pane
        self.position_cursor_to_buffer(&mut output_buffer, request_height);
        
        // Show cursor at end
        output_buffer.push_str("\x1b[?25h");
        
        // Write everything at once
        print!("{}", output_buffer);
        io::stdout().flush()?;
        Ok(())
    }

    /// TIER 2: Single pane content update (moderate cost, efficient)
    /// 
    /// Update only the pane where content changed, leaving other pane untouched.
    /// The key insight is that when typing in the Request pane, the Response pane 
    /// content is completely static and doesn't need redrawing. Without this approach,
    /// traditional methods would redraw both panes, causing flicker in the Response pane.
    /// 
    /// By isolating updates to only the active pane, we eliminate cross-pane flicker
    /// while still updating the modified content area efficiently.
    /// 
    /// WHEN USED: Content modification (typing, Enter, Backspace/Delete)
    /// PERFORMANCE: ~1-2ms - Single pane redraw with buffering  
    /// FLICKER: Minimal - Only active pane redraws, other pane stays static
    fn render_pane_update(&self) -> Result<()> {
        // Collect output for only the changed areas
        let mut output_buffer = String::new();
        
        let height = self.terminal_size.1 as usize;
        let width = self.terminal_size.0 as usize;
        
        // Calculate pane sizes using dynamic split ratio
        let request_height = if self.response_buffer.is_some() { 
            self.get_request_pane_height() 
        } else { 
            height - 1 // Full height minus status line
        };
        
        // Hide cursor at start
        output_buffer.push_str("\x1b[?25l");
        
        // Only update the active pane content
        if self.current_pane == Pane::Request {
            // Clear and redraw only the request pane area
            self.render_request_pane_to_buffer(&mut output_buffer, request_height, width);
        } else if self.current_pane == Pane::Response && self.response_buffer.is_some() {
            // Clear and redraw only the response pane area
            let response_height = self.get_response_pane_height();
            if let Some(ref response) = self.response_buffer {
                self.render_response_pane_to_buffer(&mut output_buffer, response, response_height, width, request_height + 1);
            }
        }
        
        // Update status line (minimal cost)
        self.render_status_line_to_buffer(&mut output_buffer, height - 1, width);
        
        // Position cursor based on current pane
        self.position_cursor_to_buffer(&mut output_buffer, request_height);
        
        // Show cursor at end
        output_buffer.push_str("\x1b[?25h");
        
        // Write everything at once
        print!("{}", output_buffer);
        io::stdout().flush()?;
        Ok(())
    }

    fn position_cursor(&self, request_height: usize) -> Result<()> {
        // Position cursor based on current pane
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
                let response_height = self.get_response_pane_height();
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
        Ok(())
    }

    // ========================================================================
    // BUFFER-BASED RENDERING SYSTEM
    // ========================================================================
    // 
    // Eliminate terminal flicker through atomic screen updates.
    // 
    // Direct terminal writes (print! statements) can create visual artifacts 
    // when the user sees partial screen states. By collecting all output in a 
    // String buffer first, we ensure the terminal sees only complete, consistent 
    // screen states. Without this buffering approach, users would see incomplete 
    // or inconsistent screen updates during rendering.
    // 
    // PATTERN: Each *_to_buffer method appends its output to a shared buffer.
    // The caller then writes the entire buffer atomically with a single print!
    // statement, followed by a flush to ensure immediate visibility.
    // 
    // METHODS:
    // - position_cursor_to_buffer: Cursor positioning escape sequences
    // - render_request_pane_to_buffer: Request pane content (top)
    // - render_response_pane_to_buffer: Response pane content (bottom) 
    // - render_status_line_to_buffer: Status bar (bottom line)

    fn position_cursor_to_buffer(&self, buffer: &mut String, request_height: usize) {
        // Position cursor based on current pane
        if self.current_pane == Pane::Request {
            let cursor_y = self.buffer.cursor_line.saturating_sub(self.buffer.scroll_offset);
            if cursor_y < request_height {
                // Calculate line number width and adjust cursor position
                let max_line_num = self.buffer.lines.len();
                let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
                let cursor_x = line_num_width + 1 + self.buffer.cursor_col; // +1 for space separator
                
                buffer.push_str(&format!("\x1b[{};{}H", cursor_y + 1, cursor_x + 1));
            }
        } else if self.current_pane == Pane::Response && self.response_buffer.is_some() {
            // Show cursor in response pane for navigation
            if let Some(ref response) = self.response_buffer {
                let response_height = self.get_response_pane_height();
                let cursor_y = response.cursor_line.saturating_sub(response.scroll_offset);
                if cursor_y < response_height {
                    // Calculate line number width and adjust cursor position
                    let max_line_num = response.lines.len();
                    let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
                    let cursor_x = line_num_width + 1 + response.cursor_col; // +1 for space separator
                    
                    buffer.push_str(&format!("\x1b[{};{}H", request_height + 1 + cursor_y + 1, cursor_x + 1));
                }
            }
        }
    }

    fn render_request_pane(&self, height: usize, width: usize) -> Result<()> {
        let active = self.current_pane == Pane::Request;
        
        // Calculate line number width based on total lines
        let max_line_num = self.buffer.lines.len();
        let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
        let content_width = width.saturating_sub(line_num_width + 1); // +1 for space separator
        
        for i in 0..height {
            execute!(io::stdout(), cursor::MoveTo(0, i as u16))?;
            execute!(io::stdout(), Clear(ClearType::CurrentLine))?; // Clear current line to avoid artifacts
            
            let line_idx = i + self.buffer.scroll_offset;
            
            // Render line number
            if line_idx < self.buffer.lines.len() {
                if active {
                    execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                } else {
                    execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                }
                print!("{:>width$} ", line_idx + 1, width = line_num_width);
                
                // Render line content with visual selection highlighting
                if let Some(line) = self.buffer.lines.get(line_idx) {
                    let display_line = if line.len() > content_width {
                        &line[..content_width]
                    } else {
                        line
                    };
                    
                    // Render character by character to handle visual selection
                    for (col_idx, ch) in display_line.chars().enumerate() {
                        let is_selected = self.is_position_selected(line_idx, col_idx);
                        
                        if is_selected && (self.mode == EditorMode::Visual || self.mode == EditorMode::VisualLine) {
                            // Highlight selected text
                            execute!(io::stdout(), SetBackgroundColor(Color::White))?;
                            execute!(io::stdout(), SetForegroundColor(Color::Black))?;
                        } else {
                            execute!(io::stdout(), crossterm::style::ResetColor)?;
                            if active {
                                execute!(io::stdout(), SetForegroundColor(Color::White))?;
                            } else {
                                execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                            }
                        }
                        print!("{}", ch);
                    }
                    
                    // Reset colors - line is already cleared so no need to pad
                    execute!(io::stdout(), crossterm::style::ResetColor)?;
                }
            } else {
                // Empty line - just show line number area
                execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                print!("{}", " ".repeat(line_num_width + 1));
            }
            
            execute!(io::stdout(), ResetColor)?;
        }
        
        Ok(())
    }

    fn render_request_pane_to_buffer(&self, buffer: &mut String, height: usize, width: usize) {
        let active = self.current_pane == Pane::Request;
        
        // Calculate line number width based on total lines
        let max_line_num = self.buffer.lines.len();
        let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
        let content_width = width.saturating_sub(line_num_width + 1); // +1 for space separator
        
        for i in 0..height {
            // Move to line and clear it
            buffer.push_str(&format!("\x1b[{};1H\x1b[2K", i + 1));
            
            let line_idx = i + self.buffer.scroll_offset;
            
            // Render line number
            if line_idx < self.buffer.lines.len() {
                // Line number color (dark grey)
                buffer.push_str("\x1b[90m");
                buffer.push_str(&format!("{:>width$} ", line_idx + 1, width = line_num_width));
                
                // Render line content with visual selection highlighting
                if let Some(line) = self.buffer.lines.get(line_idx) {
                    let display_line = if line.len() > content_width {
                        &line[..content_width]
                    } else {
                        line
                    };
                    
                    // Render character by character to handle visual selection
                    for (col_idx, ch) in display_line.chars().enumerate() {
                        let is_selected = self.is_position_selected(line_idx, col_idx);
                        
                        if is_selected && (self.mode == EditorMode::Visual || self.mode == EditorMode::VisualLine) {
                            // Highlight selected text
                            buffer.push_str("\x1b[47m\x1b[30m"); // White bg, black fg
                        } else {
                            buffer.push_str("\x1b[0m"); // Reset
                            if active {
                                buffer.push_str("\x1b[37m"); // White text
                            } else {
                                buffer.push_str("\x1b[90m"); // Dark grey
                            }
                        }
                        buffer.push(ch);
                    }
                    
                    // Reset colors
                    buffer.push_str("\x1b[0m");
                }
            } else {
                // Empty line - just show line number area
                buffer.push_str("\x1b[90m");
                buffer.push_str(&" ".repeat(line_num_width + 1));
                buffer.push_str("\x1b[0m");
            }
        }
    }

    fn render_response_pane(&self, response: &ResponseBuffer, height: usize, width: usize, start_row: usize) -> Result<()> {
        let active = self.current_pane == Pane::Response;
        
        // Calculate line number width based on total lines
        let max_line_num = response.lines.len();
        let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
        let content_width = width.saturating_sub(line_num_width + 1); // +1 for space separator
        
        for i in 0..height {
            execute!(io::stdout(), cursor::MoveTo(0, (start_row + i) as u16))?;
            execute!(io::stdout(), Clear(ClearType::CurrentLine))?; // Clear current line to avoid artifacts
            
            let line_idx = i + response.scroll_offset;
            
            // Render line number
            if line_idx < response.lines.len() {
                if active {
                    execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                } else {
                    execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                }
                print!("{:>width$} ", line_idx + 1, width = line_num_width);
                
                // Render line content with visual selection highlighting
                if let Some(line) = response.lines.get(line_idx) {
                    let display_line = if line.len() > content_width {
                        &line[..content_width]
                    } else {
                        line
                    };
                    
                    // Render character by character to handle visual selection
                    for (col_idx, ch) in display_line.chars().enumerate() {
                        let is_selected = self.current_pane == Pane::Response && self.is_position_selected(line_idx, col_idx);
                        
                        if is_selected && (self.mode == EditorMode::Visual || self.mode == EditorMode::VisualLine) {
                            // Highlight selected text
                            execute!(io::stdout(), SetBackgroundColor(Color::White))?;
                            execute!(io::stdout(), SetForegroundColor(Color::Black))?;
                        } else {
                            execute!(io::stdout(), crossterm::style::ResetColor)?;
                            if active {
                                execute!(io::stdout(), SetForegroundColor(Color::White))?;
                            } else {
                                execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                            }
                        }
                        print!("{}", ch);
                    }
                    
                    // Reset colors - line is already cleared so no need to pad
                    execute!(io::stdout(), crossterm::style::ResetColor)?;
                }
            } else {
                // Empty line - just show line number area
                execute!(io::stdout(), SetForegroundColor(Color::DarkGrey))?;
                print!("{}", " ".repeat(line_num_width + 1));
            }
            
            execute!(io::stdout(), ResetColor)?;
        }
        
        Ok(())
    }

    fn render_response_pane_to_buffer(&self, buffer: &mut String, response: &ResponseBuffer, height: usize, width: usize, start_row: usize) {
        let active = self.current_pane == Pane::Response;
        
        // Calculate line number width based on total lines
        let max_line_num = response.lines.len();
        let line_num_width = if max_line_num == 0 { 3 } else { format!("{}", max_line_num).len().max(3) };
        let content_width = width.saturating_sub(line_num_width + 1); // +1 for space separator
        
        for i in 0..height {
            // Move to line and clear it
            buffer.push_str(&format!("\x1b[{};1H\x1b[2K", start_row + i + 1));
            
            let line_idx = i + response.scroll_offset;
            
            // Render line number
            if line_idx < response.lines.len() {
                // Line number color (dark grey)
                buffer.push_str("\x1b[90m");
                buffer.push_str(&format!("{:>width$} ", line_idx + 1, width = line_num_width));
                
                // Render line content with visual selection highlighting
                if let Some(line) = response.lines.get(line_idx) {
                    let display_line = if line.len() > content_width {
                        &line[..content_width]
                    } else {
                        line
                    };
                    
                    // Render character by character to handle visual selection
                    for (col_idx, ch) in display_line.chars().enumerate() {
                        let is_selected = self.current_pane == Pane::Response && self.is_position_selected(line_idx, col_idx);
                        
                        if is_selected && (self.mode == EditorMode::Visual || self.mode == EditorMode::VisualLine) {
                            // Highlight selected text
                            buffer.push_str("\x1b[47m\x1b[30m"); // White bg, black fg
                        } else {
                            buffer.push_str("\x1b[0m"); // Reset
                            if active {
                                buffer.push_str("\x1b[37m"); // White text
                            } else {
                                buffer.push_str("\x1b[90m"); // Dark grey
                            }
                        }
                        buffer.push(ch);
                    }
                    
                    // Reset colors
                    buffer.push_str("\x1b[0m");
                }
            } else {
                // Empty line - just show line number area
                buffer.push_str("\x1b[90m");
                buffer.push_str(&" ".repeat(line_num_width + 1));
                buffer.push_str("\x1b[0m");
            }
        }
    }

    fn render_status_line(&self, row: usize, width: usize) -> Result<()> {
        execute!(io::stdout(), cursor::MoveTo(0, row as u16))?;
        
        // Handle command mode display
        let left_content = if self.mode == EditorMode::Command {
            format!(":{}", self.command_buffer)
        } else {
            String::new()
        };
        
        // Create right-aligned status content
        let mut right_content = String::new();
        
        if let Some(ref response_status) = self.last_response_status {
            right_content.push_str(response_status);
            
            if let Some(duration) = self.last_request_duration {
                right_content.push_str(&format!(" ({}ms)", duration));
            }
        }
        
        // Calculate available space for padding between left and right content
        let used_space = left_content.len() + right_content.len();
        let padding = if used_space < width {
            width - used_space
        } else {
            0
        };
        
        // Create the full status line
        let status_line = format!("{}{}{}", left_content, " ".repeat(padding), right_content);
        
        // Truncate if too long
        let final_status = if status_line.len() > width {
            if left_content.len() > width {
                // If command is longer than width, show only command (truncated)
                left_content[..width].to_string()
            } else {
                // Show command + as much right content as fits
                let remaining_space = width - left_content.len();
                format!("{}{}", left_content, " ".repeat(remaining_space))
            }
        } else {
            status_line
        };
        
        print!("{}", final_status);
        
        Ok(())
    }

    fn render_status_line_to_buffer(&self, buffer: &mut String, row: usize, width: usize) {
        // Move to status line position and clear it
        buffer.push_str(&format!("\x1b[{};1H\x1b[2K", row + 1));
        
        // Handle command mode display
        let left_content = if self.mode == EditorMode::Command {
            format!(":{}", self.command_buffer)
        } else {
            String::new()
        };
        
        // Create right-aligned status content
        let mut right_content = String::new();
        
        if let Some(ref response_status) = self.last_response_status {
            right_content.push_str(response_status);
            
            if let Some(duration) = self.last_request_duration {
                right_content.push_str(&format!(" ({}ms)", duration));
            }
        }
        
        // Calculate available space for padding between left and right content
        let used_space = left_content.len() + right_content.len();
        let padding = if used_space < width {
            width - used_space
        } else {
            0
        };
        
        // Create the full status line
        let status_line = format!("{}{}{}", left_content, " ".repeat(padding), right_content);
        
        // Truncate if too long
        let final_status = if status_line.len() > width {
            if left_content.len() > width {
                // If command is longer than width, show only command (truncated)
                left_content[..width].to_string()
            } else {
                // Show command + as much right content as fits
                let remaining_space = width - left_content.len();
                format!("{}{}", left_content, " ".repeat(remaining_space))
            }
        } else {
            status_line
        };
        
        buffer.push_str(&final_status);
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
            // Ensure precise ratio calculation for consistent pane boundaries to prevent
            // floating-point precision errors that would cause inconsistent pane sizes
            // when expanding to maximum bounds.
            let target_ratio = new_input_height as f64 / total_content_height as f64;
            let max_ratio = max_input_height as f64 / total_content_height as f64;
            self.pane_split_ratio = target_ratio.min(max_ratio);
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
            // Maintain precise ratio calculation when expanding output pane to ensure
            // consistent behavior with input pane resizing and prevent boundary precision
            // issues that would cause the pane to not reach maximum output pane size.
            let target_ratio = new_input_height as f64 / total_content_height as f64;
            let min_ratio = min_input_height as f64 / total_content_height as f64;
            self.pane_split_ratio = target_ratio.max(min_ratio);
        }
    }

    fn shrink_input_pane(&mut self) {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let min_input_height = 3;
        let current_input_height = self.get_request_pane_height();
        
        if current_input_height > min_input_height {
            let new_input_height = (current_input_height - 1).max(min_input_height);
            // Ensure precise ratio calculation to avoid floating-point precision issues
            // that would cause the pane to get stuck at values like 4 lines instead of
            // reaching the exact minimum of 3 lines. We calculate the exact ratio and
            // clamp it to ensure boundaries are respected.
            let target_ratio = new_input_height as f64 / total_content_height as f64;
            let min_ratio = min_input_height as f64 / total_content_height as f64;
            self.pane_split_ratio = target_ratio.max(min_ratio);
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
            // Ensure precise ratio calculation when shrinking output pane to prevent
            // the output pane from getting stuck above the minimum size due to
            // floating-point precision errors in ratio calculations.
            let target_ratio = new_input_height as f64 / total_content_height as f64;
            let max_ratio = (total_content_height - min_output_height) as f64 / total_content_height as f64;
            self.pane_split_ratio = target_ratio.min(max_ratio);
        }
    }

    fn maximize_input_pane(&mut self) {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let min_output_height = 3;
        let max_input_height = total_content_height - min_output_height;
        
        self.pane_split_ratio = max_input_height as f64 / total_content_height as f64;
    }

    fn maximize_output_pane(&mut self) {
        let height = self.terminal_size.1 as usize;
        let total_content_height = height - 2; // Minus separator and status line
        let min_input_height = 3;
        
        self.pane_split_ratio = min_input_height as f64 / total_content_height as f64;
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
