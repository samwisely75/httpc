# httpc

[![License](https://img.shields.io/badge/license-Elastic%20License%202.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/github/v/release/samwisely75/httpc)](https://github.com/samwisely75/httpc/releases)
[![CI](https://github.com/samwisely75/httpc/actions/workflows/ci.yml/badge.svg)](https://github.com/samwisely75/httpc/actions/workflows/ci.yml)
[![Release](https://github.com/samwisely75/httpc/actions/workflows/release.yml/badge.svg)](https://github.com/samwisely75/httpc/actions/workflows/release.yml)

A lightweight, profile-based HTTP client that allows you to talk to web servers with minimal effort. Think of it as `curl` with persistent profile and simplified syntax.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Configuration](#configuration)
- [Quick Start](#quick-start)
- [Usage](#usage)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)
- [Why httpc and not curl?](#why-httpc-and-not-curl)
- [License](#license)

## Features

**Profile-based simplicity** - Transform complex curl commands into simple, memorable requests. Store connection details, authentication, and headers in `~/.httpc/profile` once, then use clean relative URLs like `httpc -p prod GET /api/users` instead of repeating lengthy curl parameters every time.

Plus all the HTTP client features you expect:

- **Multiple HTTP methods** - Support GET, POST, PUT, DELETE, HEAD, etc.
- **Authentication support** - Basic auth, custom headers
- **SSL/TLS support** - Custom CA certificates, insecure mode option
- **Proxy support** - HTTP proxy configuration  
- **Standard input support** - Read request body from stdin
- **Multiple compression** - gzip, deflate, zstd
- **Flexible URL handling** - Absolute or relative URLs with profile base
- **Verbose mode** - Detailed request/response information

## Installation

Download the appropriate binary from [releases](https://github.com/samwisely75/httpc/releases) for your platform:

**macOS (Homebrew):**

```bash
# Install via Homebrew (easiest method for macOS)
brew install samwisely75/tap/httpc
```

**Linux/macOS (Manual):**

```bash
# Download and extract
curl -L https://github.com/samwisely75/httpc/releases/latest/download/httpc-linux-x64.tar.gz | tar -xz
sudo mv httpc /usr/local/bin/

# Or for macOS
curl -L https://github.com/samwisely75/httpc/releases/latest/download/httpc-macos-x64.tar.gz | tar -xz
sudo mv httpc /usr/local/bin/
```

**From crates.io (requires Rust):**

```bash
cargo install httpc
```

**Build from source:**

```bash
git clone https://github.com/samwisely75/httpc.git
cd httpc
cargo build --release
sudo cp target/release/httpc /usr/local/bin/
```

Test the installation: `httpc --help`

No additional dependencies required - httpc is a single, self-contained binary.

## Configuration

### Configuration File Location

httpc looks for configuration in `~/.httpc/profile`.

### Configuration Format

The configuration file uses INI format with multiple profiles. Each profile contains connection details and default headers.

```ini
[default]
host = https://api.example.com
user = your-username
password = your-password
insecure = false
ca_cert = /path/to/ca.pem
@content-type = application/json
@user-agent = httpc/0.1
@accept = application/json
@accept-encoding = gzip, deflate

[staging]
host = https://staging-api.example.com  
user = staging-user
password = staging-pass
@content-type = application/json

[local]
host = http://localhost:8080
user = admin
password = admin
```

### Configuration Options

#### Connection Settings

- `host` - Base URL for requests (required for relative URLs)
- `user` - Username for basic authentication
- `password` - Password for basic authentication
- `ca_cert` - Path to CA certificate file for SSL/TLS
- `insecure` - Skip SSL/TLS certificate verification (true/false)

#### HTTP Headers

Any key starting with `@` becomes an HTTP header. Please feel free to add any custom headers you need.

Examples:

- `@content-type` → `Content-Type` header
- `@authorization` → `Authorization` header
- `@user-agent` → `User-Agent` header
- `@accept` → `Accept` header

### Profile Selection

```bash
# Use default profile
httpc GET /api/endpoint

# Use specific profile
httpc -p staging GET /api/endpoint
httpc --profile production GET /api/endpoint
```

### Override with Command Line Options

Command line options override profile settings:

```bash
# Override user from profile
httpc -p staging --user different-user GET /api/data

# Override host entirely
httpc GET https://completely-different-host.com/api
```

## Quick Start

1. **Create a profile for your favorite API:**

   ```bash
   # Create ~/.httpc directory and profile file
   mkdir -p ~/.httpc
   echo "[swapi]
   host = https://swapi.dev/api
   @content-type = application/json" > ~/.httpc/profile
   ```

2. **Use the profile to explore the Star Wars universe:**

   ```bash
   # Now you can use short URLs to explore the galaxy!
   httpc -p swapi GET /people/1/        # Luke Skywalker
   httpc -p swapi GET /people/4/        # Darth Vader
   httpc -p swapi GET /starships/10/    # Millennium Falcon
   httpc -p swapi GET /films/1/         # A New Hope
   ```

3. **Or override with full URLs when needed:**

   ```bash
   # You can always use absolute URLs to override the profile
   httpc GET https://httpbin.org/get
   httpc GET https://swapi.dev/api/planets/
   ```

## Usage

### Basic HTTP Requests

The simplest usage is to make a request to any URL:

```bash
# GET request
httpc GET https://httpbin.org/get

# POST request with JSON data
httpc POST https://httpbin.org/post '{
    "name": "John Doe",
    "email": "john@example.com"
}'

# PUT request
httpc PUT https://httpbin.org/put '{"status": "updated"}'

# DELETE request
httpc DELETE https://httpbin.org/delete
```

### Using Standard Input

You can pass request body via standard input:

```bash
# From file
cat data.json | httpc POST https://api.example.com/users

# From command output
echo '{"query": {"match_all": {}}}' | httpc POST https://elasticsearch.example.com/my-index/_search

# Complex pipeline example
echo '{
    "query": {
        "range": {
            "@timestamp": {
                "gte": "now-1d/d",
                "lt": "now/d"
            }
        }
    }
}' | httpc GET my-index/_search | jq '.hits.hits[]._source.name'
```

### Profile-Based Requests

Use profile to avoid repeating connection details:

```bash
# Use default profile
httpc GET /api/users

# Use specific profile
httpc -p staging GET /api/users
httpc -p production GET /health
```

### Authentication & Headers

```bash
# Basic authentication
httpc GET https://api.example.com/protected \
    --user admin \
    --password secret

# Custom headers
httpc POST https://api.example.com/data \
    -H "Authorization: Bearer your-token" \
    -H "X-Custom-Header: value" \
    '{"data": "value"}'

# SSL options
httpc GET https://self-signed.example.com/api \
    --ca-cert /path/to/ca.pem \
    --insecure
```

### Advanced Usage

```bash
# Through proxy
httpc GET https://api.example.com/data \
    --proxy http://proxy.company.com:8080

# Verbose mode for debugging
httpc -v GET https://api.example.com/debug

# Override profile settings
httpc -p production GET /api/data \
    --user different-user \
    --password different-pass
```

### Shell Escaping

When using special characters in URLs or query parameters, you may need to escape them or use quotes:

```bash
# Query parameters with special characters - escape or quote
httpc GET "/api/search?q=hello world&sort=date"
httpc GET /api/search\?q=hello\ world\&sort=date

# Complex URLs with fragments
httpc GET "https://api.example.com/items?filter=status:active&limit=10#results"

# JSON with special characters in URLs
httpc POST "/api/items?category=tools&type=screws" '{
    "name": "Phillips head screw",
    "size": "M4"
}'
```

**Tip:** When in doubt, wrap URLs in double quotes to avoid shell interpretation issues.

For all available options, run:

```bash
httpc --help
```

## Examples

### API Testing

```bash
# Test REST API endpoints
httpc GET https://jsonplaceholder.typicode.com/posts/1
httpc POST https://jsonplaceholder.typicode.com/posts '{
    "title": "My Post",
    "body": "Post content",
    "userId": 1
}'
```

### Working with Different Content Types

```bash
# JSON API
httpc POST https://api.example.com/users \
    -H "Content-Type: application/json" \
    '{"name": "John", "email": "john@example.com"}'

# Form data
httpc POST https://api.example.com/form \
    -H "Content-Type: application/x-www-form-urlencoded" \
    'name=John&email=john@example.com'

# File upload simulation
cat document.json | httpc PUT https://api.example.com/documents/123
```

### Elasticsearch Examples

```bash
# Check cluster health
httpc -p elastic GET /_cluster/health

# Search documents
echo '{
    "query": {
        "match": {"title": "search term"}
    }
}' | httpc -p elastic GET /my-index/_search

# Index a document
httpc -p elastic PUT /my-index/_doc/1 '{
    "title": "My Document",
    "content": "Document content here"
}'
```

### Development Workflow

```bash
# Set up profile for different environments
mkdir -p ~/.httpc
cat > ~/.httpc/profile << EOF
[dev]
host = http://localhost:3000
@content-type = application/json

[staging]
host = https://staging-api.company.com
user = api-user
password = staging-password
@authorization = Bearer staging-token

[prod]
host = https://api.company.com
user = api-user
password = production-password
@authorization = Bearer production-token
EOF

# Test the same endpoint across environments
httpc -p dev GET /api/health
httpc -p staging GET /api/health
httpc -p prod GET /api/health
```

## Troubleshooting

### Common Issues

**Q: `httpc: command not found`**
A: Make sure httpc is in your PATH. Try `which httpc` or reinstall following the installation instructions.

**Q: SSL certificate errors**
A: Use `--insecure` to skip certificate validation, or provide a CA certificate with `--ca-cert /path/to/ca.pem`.

**Q: Profile not found**
A: Check that `~/.httpc/profile` exists and contains the profile. Use `httpc -p nonexistent GET /` to see the error.

**Q: Authentication failures**
A: Verify credentials in your profile or override with `--user` and `--password` flags.

**Q: Request body from stdin not working**
A: Make sure you're piping data correctly: `echo '{"key": "value"}' | httpc POST /api/endpoint`

### Debug Mode

Use verbose mode to see detailed request/response information:

```bash
httpc -v GET https://httpbin.org/get
```

This shows:

- Full request URL and headers
- Response status and headers
- Timing information
- SSL/TLS details

### Configuration Validation

Check your configuration file:

```bash
# View current configuration
cat ~/.httpc/profile

# Test with a simple request
httpc -p your-profile GET /simple/endpoint
```

## Why httpc and not curl?

I work with Elasticsearch clusters day in and day out. Kibana Dev Tools is ideal, but often unavailable in client environments where I need to SSH into nodes, check logs, and run diagnostic queries from the terminal.

`curl` works, but it becomes tedious with repetitive parameters:

```bash
curl -XGET -u elastic:password -H "content-type: application/json" \
     https://elastic-prod.es.us-central1.gcp.cloud.es.io/_cat/indices?v
```

In Kibana Dev Tools, this is simply:

```bash
GET /_cat/indices?v
```

I wanted that same simplicity in the terminal by bringing a profile system into it for easily switching between multiple clusters like `aws-cli`.

### Why Rust?

Python and Bash scripts work but become unwieldy and hard to maintain. Sometimes I even need to work with Python 2.6/2.7 that complicate the codes due to the compatibility issues. Rust works perfectly as:

- **Single binary** - No runtime dependencies, easy deployment
- **Performance** - Native speed, as fast as curl
- **Cross-platform** - Works on Linux and macOS
- **Reliability** - Memory safety and robust error handling
- **No compatibility hell** - Unlike Python scripts with version dependencies

---

## Contributing

### Development Setup

1. **Prerequisites**

   ```bash
   # Install Rust toolchain
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Build from source**

   ```bash
   git clone https://github.com/samwisely75/httpc.git
   cd httpc
   cargo build
   ```

### Code Quality Tools

For contributors working on the codebase, we've set up automated tooling to maintain code quality:

```bash
# Automatically fix formatting and linting issues
make fix

# Or manually run individual steps:
cargo fmt                           # Format code
cargo clippy --fix --allow-dirty    # Fix clippy warnings
cargo test                          # Run tests
```

The `make fix` command will:

- ✨ Format all code with `rustfmt`
- 🔧 Automatically fix clippy warnings (like `uninlined_format_args`)
- ✅ Verify all lints pass
- 🧪 Run the test suite to ensure nothing broke

**Pro tip:** Run `make fix` before committing to ensure your code passes CI!

### Other Make Targets

```bash
make help           # Show all available targets
make build          # Build the project  
make test           # Run tests
make lint           # Check linting only
make fmt            # Format code only
make check          # Run format + lint + test
```

---

## License

Licensed under the Elastic License 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at:

[https://www.elastic.co/licensing/elastic-license](https://www.elastic.co/licensing/elastic-license)

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
