# webly

[![CI](https://github.com/blueeaglesam/webly/actions/workflows/ci.yml/badge.svg)](https://github.com/blueeaglesam/webly/actions/workflows/ci.yml)
[![Release](https://github.com/blueeaglesam/webly/actions/workflows/release.yml/badge.svg)](https://github.com/blueeaglesam/webly/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/webly.svg)](https://crates.io/crates/webly)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/webly.svg)](https://crates.io/crates/webly)

A lightweight, profile-based HTTP client that allows you to talk to web servers with minimal effort. Think of it as `curl` with persistent profiles and simplified syntax.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage](#usage)
- [Configuration](#configuration)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [License](#license)

## Usage

### Basic HTTP Requests

The simplest usage is to make a request to any URL:

```bash
# GET request
webly GET https://httpbin.org/get

# POST request with JSON data
webly POST https://httpbin.org/post '{
    "name": "John Doe",
    "email": "john@example.com"
}'

# PUT request
webly PUT https://httpbin.org/put '{"status": "updated"}'

# DELETE request
webly DELETE https://httpbin.org/delete
```

### Using Standard Input

You can pass request body via standard input:

```bash
# From file
cat data.json | webly POST https://api.example.com/users

# From command output
echo '{"query": {"match_all": {}}}' | webly POST https://elasticsearch.example.com/my-index/_search

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
}' | webly GET my-index/_search | jq '.hits.hits[]._source.name'
```

### Profile-Based Requests

Use profiles to avoid repeating connection details:

```bash
# Use default profile
webly GET /api/users

# Use specific profile
webly -p staging GET /api/users
webly -p production GET /health
```

### Authentication & Headers

```bash
# Basic authentication
webly GET https://api.example.com/protected \
    --user admin \
    --password secret

# Custom headers
webly POST https://api.example.com/data \
    -H "Authorization: Bearer your-token" \
    -H "X-Custom-Header: value" \
    '{"data": "value"}'

# SSL options
webly GET https://self-signed.example.com/api \
    --ca-cert /path/to/ca.pem \
    --insecure
```

### Advanced Usage

```bash
# Through proxy
webly GET https://api.example.com/data \
    --proxy http://proxy.company.com:8080

# Verbose mode for debugging
webly -v GET https://api.example.com/debug

# Override profile settings
webly -p production GET /api/data \
    --user different-user \
    --password different-pass
```

For all available options, run:

```bash
webly --help
```

## Features

- **Profile-based configuration** - Store connection details in `~/.webly` for reuse
- **Multiple HTTP methods** - Support GET, POST, PUT, DELETE, HEAD, etc.
- **Authentication support** - Basic auth, custom headers
- **SSL/TLS support** - Custom CA certificates, insecure mode option
- **Proxy support** - HTTP proxy configuration  
- **Standard input support** - Read request body from stdin
- **Multiple compression** - gzip, deflate, zstd
- **Flexible URL handling** - Absolute or relative URLs with profile base
- **Verbose mode** - Detailed request/response information
- **Auto redirection** - Automatic following of HTTP redirects

## Installation

### From Pre-built Binaries

1. Download the binary from [releases](https://github.com/blueeaglesam/webly/releases) for your platform
2. Extract the `.tar.gz` file: `tar -xzf webly-*.tar.gz`
3. Copy `webly` to a directory in your `$PATH` (e.g., `/usr/local/bin`)
4. Test the installation: `webly --help`

### From Source (Rust Required)

```bash
# Install from crates.io
cargo install webly

# Or build from source
git clone https://github.com/blueeaglesam/webly.git
cd webly
cargo build --release
sudo cp target/release/webly /usr/local/bin/
```

### System Requirements

- macOS, Linux, or Windows
- No additional dependencies required

## Quick Start

1. **Simple GET request to any URL:**
   ```bash
   webly GET https://httpbin.org/get
   ```

2. **Create a profile for repeated use:**
   ```bash
   # Create ~/.webly file
   echo "[api]
   host = https://api.example.com
   user = your-username
   password = your-password
   @content-type = application/json" > ~/.webly
   ```

3. **Use the profile:**
   ```bash
   webly -p api GET /users/me
   ```

## Configuration

### Configuration File Location

webly looks for configuration in `~/.webly` (on Unix/Linux/macOS) or `%USERPROFILE%\.webly` (on Windows).

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
@user-agent = webly/0.1
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
Any key starting with `@` becomes an HTTP header:
- `@content-type` → `Content-Type` header
- `@authorization` → `Authorization` header
- `@user-agent` → `User-Agent` header
- `@accept` → `Accept` header

### Profile Selection

```bash
# Use default profile
webly GET /api/endpoint

# Use specific profile
webly -p staging GET /api/endpoint
webly --profile production GET /api/endpoint
```

### Command Line Override

Command line options override profile settings:

```bash
# Override user from profile
webly -p staging --user different-user GET /api/data

# Override host entirely
webly GET https://completely-different-host.com/api
``` 

## Examples

### API Testing

```bash
# Test REST API endpoints
webly GET https://jsonplaceholder.typicode.com/posts/1
webly POST https://jsonplaceholder.typicode.com/posts '{
    "title": "My Post",
    "body": "Post content",
    "userId": 1
}'
```

### Working with Different Content Types

```bash
# JSON API
webly POST https://api.example.com/users \
    -H "Content-Type: application/json" \
    '{"name": "John", "email": "john@example.com"}'

# Form data
webly POST https://api.example.com/form \
    -H "Content-Type: application/x-www-form-urlencoded" \
    'name=John&email=john@example.com'

# File upload simulation
cat document.json | webly PUT https://api.example.com/documents/123
```

### Elasticsearch/OpenSearch Examples

```bash
# Check cluster health
webly -p elastic GET /_cluster/health

# Search documents
echo '{
    "query": {
        "match": {"title": "search term"}
    }
}' | webly -p elastic GET /my-index/_search

# Index a document
webly -p elastic PUT /my-index/_doc/1 '{
    "title": "My Document",
    "content": "Document content here"
}'
```

### Development Workflow

```bash
# Set up profiles for different environments
cat > ~/.webly << EOF
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
webly -p dev GET /api/health
webly -p staging GET /api/health
webly -p prod GET /api/health
```

## Troubleshooting

### Common Issues

**Q: `webly: command not found`**
A: Make sure webly is in your PATH. Try `which webly` or reinstall following the installation instructions.

**Q: SSL certificate errors**
A: Use `--insecure` to skip certificate validation, or provide a CA certificate with `--ca-cert /path/to/ca.pem`.

**Q: Profile not found**
A: Check that `~/.webly` exists and contains the profile. Use `webly -p nonexistent GET /` to see the error.

**Q: Authentication failures**
A: Verify credentials in your profile or override with `--user` and `--password` flags.

**Q: Request body from stdin not working**
A: Make sure you're piping data correctly: `echo '{"key": "value"}' | webly POST /api/endpoint`

### Debug Mode

Use verbose mode to see detailed request/response information:

```bash
webly -v GET https://httpbin.org/get
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
cat ~/.webly

# Test with a simple request
webly -p your-profile GET /simple/endpoint
```

## Motivation

As a consultant working with APIs daily, especially Elasticsearch, I found that existing tools had limitations:

**Kibana Dev Tools** is excellent but not always available in client environments.

**curl** works everywhere but becomes cumbersome with repetitive parameters:

```bash
curl -XGET \
     -u "elastic:password" \
     -H "content-type: application/json" \
     https://prod-cluster.es.us-central1.gcp.cloud.es.io/_cat/indices?v
```

**What I wanted** was the simplicity of Kibana Dev Tools:

```bash
GET /_cat/indices?v
```

**webly bridges this gap** by combining curl's universality with profile-based simplicity:

```bash
webly GET /_cat/indices?v  # Uses default profile
```

### Why Rust?

- **Single binary** - No runtime dependencies, easy to deploy
- **Cross-platform** - Works on Linux, macOS, Windows  
- **Performance** - Fast startup and execution
- **Reliability** - Memory safety and robust error handling
- **Maintainability** - Strong type system prevents many bugs

Python scripts become unwieldy for this use case, requiring multiple files and dependency management. Rust delivers a professional tool that "just works" everywhere.

---

**Repository:** [https://github.com/blueeaglesam/webly](https://github.com/blueeaglesam/webly)

## License

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at:

[http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0)

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.

## Contributing

Contributions are welcome! Here's how you can help:

### Reporting Issues

- Use the [GitHub issue tracker](https://github.com/blueeaglesam/webly/issues)
- Provide detailed reproduction steps
- Include webly version: `webly --version`
- Include OS and environment details

### Development Setup

```bash
# Clone the repository
git clone https://github.com/blueeaglesam/webly.git
cd webly

# Build the project
cargo build

# Run tests
cargo test

# Run with example
cargo run -- GET https://httpbin.org/get
```

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass: `cargo test`
6. Update documentation if needed
7. Submit a pull request

### Enhancement Ideas

- [ ] Support for client certificate authentication
- [ ] Binary data handling improvements
- [ ] Multi-form POST support
- [ ] REPL mode with session/cookie support
- [ ] JSON response beautification
- [ ] Response time measurements
- [ ] HTTP/2 support
- [ ] Configuration file validation
- [ ] Shell completion scripts

