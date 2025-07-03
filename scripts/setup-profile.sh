#!/bin/bash
# Setup script for webly - creates initial ~/.webly/profile configuration

set -e

# Create webly config directory
CONFIG_DIR="$HOME/.webly"
PROFILE_PATH="$CONFIG_DIR/profile"

# Create directory if it doesn't exist
if [ ! -d "$CONFIG_DIR" ]; then
    mkdir -p "$CONFIG_DIR"
    echo "Created webly config directory: $CONFIG_DIR"
fi

# Create initial profile file if it doesn't exist
if [ ! -f "$PROFILE_PATH" ]; then
    cat > "$PROFILE_PATH" << 'EOF'
# Webly profile Configuration
# 
# This file contains profile definitions for the webly HTTP client.
# Each profile section defines connection settings and default headers.
#
# Example profile:

[httpbin]
host = https://httpbin.org
@content-type = application/json
@user-agent = webly/0.1.7

[jsonplaceholder]
host = https://jsonplaceholder.typicode.com
@content-type = application/json

[localhost]
host = http://localhost:3000
@content-type = application/json

# Add your own profile here:
# [myapi]
# host = https://api.example.com
# @authorization = Bearer your-token-here
# @content-type = application/json
EOF
    echo "Created initial profile configuration: $PROFILE_PATH"
    echo ""
    echo "Example usage:"
    echo "  webly -p httpbin GET /get"
    echo "  webly -p jsonplaceholder GET /posts/1"
    echo ""
    echo "Edit $PROFILE_PATH to add your own API profile."
else
    echo "profile file already exists: $PROFILE_PATH"
fi
