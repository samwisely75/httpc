# Setup script for webly - creates initial ~/.webly/profiles configuration
# PowerShell version for Windows

$ErrorActionPreference = "Stop"

# Create webly config directory
$ConfigDir = Join-Path $env:USERPROFILE ".webly"
$ProfilesFile = Join-Path $ConfigDir "profiles"

# Create directory if it doesn't exist
if (-not (Test-Path $ConfigDir)) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    Write-Host "Created webly config directory: $ConfigDir"
}

# Create initial profiles file if it doesn't exist
if (-not (Test-Path $ProfilesFile)) {
    $ProfilesContent = @'
# Webly Profiles Configuration
# 
# This file contains profile definitions for the webly HTTP client.
# Each profile section defines connection settings and default headers.
#
# Example profiles:

[httpbin]
host = https://httpbin.org
@content-type = application/json
@user-agent = webly/0.1.6

[jsonplaceholder]
host = https://jsonplaceholder.typicode.com
@content-type = application/json

[localhost]
host = http://localhost:3000
@content-type = application/json

# Add your own profiles here:
# [myapi]
# host = https://api.example.com
# @authorization = Bearer your-token-here
# @content-type = application/json
'@

    $ProfilesContent | Out-File -FilePath $ProfilesFile -Encoding UTF8
    Write-Host "Created initial profiles configuration: $ProfilesFile"
    Write-Host ""
    Write-Host "Example usage:"
    Write-Host "  webly -p httpbin GET /get"
    Write-Host "  webly -p jsonplaceholder GET /posts/1"
    Write-Host ""
    Write-Host "Edit $ProfilesFile to add your own API profiles."
} else {
    Write-Host "Profiles file already exists: $ProfilesFile"
}

# Pause to show the message before closing
Write-Host ""
Write-Host "Press any key to continue..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
