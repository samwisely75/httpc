@echo off
REM Setup script for webly - creates initial ~/.webly/profiles configuration

echo Setting up webly...

REM Create webly config directory
set CONFIG_DIR=%USERPROFILE%\.webly
set PROFILES_FILE=%CONFIG_DIR%\profiles

REM Create directory if it doesn't exist
if not exist "%CONFIG_DIR%" (
    mkdir "%CONFIG_DIR%"
    echo Created webly config directory: %CONFIG_DIR%
)

REM Create initial profiles file if it doesn't exist
if not exist "%PROFILES_FILE%" (
    (
        echo # Webly Profiles Configuration
        echo # 
        echo # This file contains profile definitions for the webly HTTP client.
        echo # Each profile section defines connection settings and default headers.
        echo #
        echo # Example profiles:
        echo.
        echo [httpbin]
        echo host = https://httpbin.org
        echo @content-type = application/json
        echo @user-agent = webly/0.1.7
        echo.
        echo [jsonplaceholder]
        echo host = https://jsonplaceholder.typicode.com
        echo @content-type = application/json
        echo.
        echo [localhost]
        echo host = http://localhost:3000
        echo @content-type = application/json
        echo.
        echo # Add your own profiles here:
        echo # [myapi]
        echo # host = https://api.example.com
        echo # @authorization = Bearer your-token-here
        echo # @content-type = application/json
    ) > "%PROFILES_FILE%"
    echo Created initial profiles configuration: %PROFILES_FILE%
    echo.
    echo Example usage:
    echo   webly -p httpbin GET /get
    echo   webly -p jsonplaceholder GET /posts/1
    echo.
    echo Edit %PROFILES_FILE% to add your own API profiles.
) else (
    echo Profiles file already exists: %PROFILES_FILE%
)

echo Webly setup complete!
echo See documentation: https://github.com/blueeaglesam/webly
pause
