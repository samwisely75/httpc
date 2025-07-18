# REPL Mode Documentation

The `httpc` HTTP client includes a powerful REPL (Read-Eval-Print Loop) mode that provides a vim-like terminal user interface for interactively testing HTTP requests.

## Getting Started

Start the REPL by running the application without arguments:

```bash
./httpc
```

The application will enter REPL mode with a dual-pane interface.

## Interface Overview

The REPL provides a split-screen terminal interface with two main panes:

- **Request Pane** (Top): Where you compose HTTP requests (editable)
- **Response Pane** (Bottom): Displays HTTP responses (read-only with full navigation)
- **Status Line** (Bottom): Shows command input and response status with timing

### Line Numbers

Both panes display line numbers on the left side:
- Line numbers are 1-based (start from 1)
- Width automatically adjusts based on total number of lines
- Displayed in subtle dark grey color

## Editor Modes

The REPL supports multiple vim-like editing modes:

### Insert Mode
- **Entry**: Press `i` (insert), `I` (insert at line start), `A` (append at line end)
- **Features**: Normal text editing with character insertion
- **Exit**: Press `Esc` to return to Normal mode
- **Indicator**: Shows `-- INSERT --` in status line

### Normal Mode
- **Default mode** for navigation and commands
- **Features**: Cursor movement, text manipulation, request execution
- **Indicator**: Shows `NORMAL` in status line

### Command Mode
- **Entry**: Press `:` from Normal mode
- **Features**: Execute commands like `:q` (quit), `:w` (execute request)
- **Exit**: Press `Esc` or `Enter`

### Visual Mode
- **Entry**: Press `v` (character visual) or `V` (line visual)
- **Features**: Text selection for copy/cut operations
- **Operations**: `y` (yank), `d` (delete)

## Navigation

### Basic Movement
- **h, j, k, l** or **Arrow Keys**: Move cursor left, down, up, right (works in both panes)
- **w**: Move forward by word (works in both panes)
- **b**: Move backward by word (works in both panes)
- **0**: Move to beginning of line (works in both panes)
- **$**: Move to end of line (works in both panes)
- **gg**: Go to start of buffer (works in both panes)
- **G**: Go to end of buffer (works in both panes)

### Scrolling
- **Ctrl+U**: Scroll up half page
- **Ctrl+D**: Scroll down half page
- **j/k at boundaries**: Auto-scroll when cursor reaches top/bottom
- **Page Up/Page Down**: Scroll by half page

### Auto-Scrolling
- **New lines**: Automatically scroll to keep cursor visible when adding lines
- **Cursor movement**: Auto-scroll when cursor moves beyond visible area
- **Smart boundaries**: Prevents content from disappearing under pane boundaries

## Pane Management

### Pane Switching
- **Tab**: Switch between Request and Response panes
- **Ctrl+W w**: Switch to next pane
- **Ctrl+W h**: Move to Request pane
- **Ctrl+W l**: Move to Response pane
- **Ctrl+W j/k**: Toggle between panes

### Dynamic Pane Resizing
The pane boundary can be dynamically adjusted:

#### Boundary Control Keys
- **Ctrl+K**: Move boundary upward (shrink Request / expand Response)
- **Ctrl+J**: Move boundary downward (expand Request / shrink Response) 
- **Ctrl+M**: Maximize current pane

#### Constraints
- Minimum pane height: 3 lines
- Automatic adjustment to maintain visibility
- Boundary controls work in both Normal and Insert modes

## HTTP Request Composition

### Request Format
Compose HTTP requests in the Request pane using this format:

```
METHOD URL
Header-Name: Header-Value
Another-Header: Value

Request Body (JSON, text, etc.)
```

### Example Request
```
GET https://jsonplaceholder.typicode.com/posts/1
Accept: application/json
User-Agent: httpc/1.0

```

### Supported Methods
- GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS
- Case-insensitive method names

## Request Execution

### Execution Methods
- **Enter** (Normal mode): Execute HTTP request
- **Ctrl+Enter** (Insert mode): Execute request without leaving Insert mode
- **:w** (Command mode): Execute request
- **:wq** (Command mode): Execute request and quit

### Response Display
Responses appear in the Response pane with:
- HTTP status code and message
- Response headers (in verbose mode)
- JSON responses (pretty-printed)
- Plain text responses

## Text Operations

### Copy/Paste (Yanking)
- **y**: Yank (copy) current line
- **Y**: Yank from cursor to end of line
- **p**: Paste below current line
- **P**: Paste above current line

### Deletion
- **x**: Delete character at cursor
- **d**: Delete current line
- **D**: Delete from cursor to end of line
- **Backspace**: Delete character before cursor

### Line Operations
- **J**: Join current line with next line
- **Enter** (Insert mode): Create new line with auto-scroll

## Session Management

### Headers
Set session-wide headers using commands:
```
:set Authorization Bearer your-token-here
:set Content-Type application/json
```

### Verbose Mode
Toggle detailed response headers:
```
:verbose
```

### Clear Response
Clear the response pane:
```
:clear
```

## Advanced Features

### Visual Selection
1. Enter Visual mode with `v` or `V`
2. Move cursor to select text
3. Perform operations:
   - `y`: Copy selection
   - `d`: Cut selection

### Command History
- Commands are preserved within session
- Headers persist across requests

### Error Handling
- Invalid requests show error messages
- Network errors displayed in Response pane
- Graceful handling of malformed URLs

## Keyboard Shortcuts Reference

| Key Combination | Mode | Action |
|----------------|------|--------|
| `i` | Normal | Enter Insert mode |
| `Esc` | Insert/Command | Return to Normal mode |
| `:` | Normal | Enter Command mode |
| `Tab` | Any | Switch panes |
| `Ctrl+W w` | Any | Switch panes (vim-style) |
| `Ctrl+K` | Any | Shrink Input / Expand Output |
| `Ctrl+J` | Any | Expand Input / Shrink Output |
| `Ctrl+M` | Any | Maximize current pane |
| `j/k` | Normal | Move with auto-scroll at boundaries |
| `Ctrl+U/D` | Normal | Scroll half page |
| `Enter` | Normal | Execute HTTP request |
| `Ctrl+Enter` | Insert | Execute HTTP request |

## Commands Reference

| Command | Action |
|---------|--------|
| `:q` | Quit application (or hide Response pane if in Response pane) |
| `:quit` | Same as `:q` |
| `:w` | Execute HTTP request |
| `:write` | Same as `:w` |
| `:wq` | Execute request and quit |
| `:clear` | Clear response buffer |
| `:verbose` | Toggle verbose mode |
| `:set key value` | Set session header |

## Tips and Best Practices

1. **Start with GET requests** to test connectivity
2. **Use session headers** for authentication tokens
3. **Enable verbose mode** to debug header issues
4. **Use pane resizing** to focus on input or output as needed
5. **Leverage auto-scrolling** when composing long requests
6. **Use visual mode** for precise text selection and manipulation

## Troubleshooting

### Common Issues
- **Cursor disappears**: Check if cursor is outside visible pane area
- **No response**: Verify URL format and network connectivity
- **Headers not working**: Use `:set` command syntax correctly
- **Pane too small**: Use `Ctrl+J/K/M` to adjust pane sizes

### Performance
- Large responses may take time to render
- Use `:clear` to free memory after large responses
- Line numbers adapt to content size automatically

## Technical Details

### Terminal Compatibility
- Requires terminal with ANSI color support
- Works with most modern terminal emulators
- Crossterm library handles platform differences

### Memory Management
- Responses stored in memory during session
- Use `:clear` command to free response buffers
- Auto-scrolling optimized for performance

---

This REPL mode transforms `httpc` into a powerful interactive HTTP testing tool, combining the efficiency of vim-like editing with modern HTTP client capabilities.
