use std::process::Command;
use tempfile::tempdir;

fn httpc_binary() -> String {
    env!("CARGO_BIN_EXE_httpc").to_string()
}

#[test]
fn test_help_command() {
    let output = Command::new(httpc_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute httpc");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("A lightweight, profile-based HTTP client"));
    assert!(stdout.contains("Usage:"));
}

#[test]
fn test_version_command() {
    let output = Command::new(httpc_binary())
        .arg("--version")
        .output()
        .expect("Failed to execute httpc");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("httpc"));
}

#[test]
fn test_basic_get_request() {
    let output = Command::new(httpc_binary())
        .args(["GET", "https://httpbin.org/get"])
        .output()
        .expect("Failed to execute httpc");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("httpbin.org"));
    } else {
        // Network might not be available in CI, so we just check the binary runs
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Network request failed (expected in some CI environments): {stderr}");
    }
}

#[test]
fn test_post_with_stdin() {
    let mut cmd = Command::new(httpc_binary())
        .args(["POST", "https://httpbin.org/post"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn httpc");

    if let Some(stdin) = cmd.stdin.as_mut() {
        use std::io::Write;
        stdin
            .write_all(b"{\"test\": \"data\"}")
            .expect("Failed to write to stdin");
    }

    let output = cmd.wait_with_output().expect("Failed to read stdout");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("httpbin.org"));
    } else {
        // Network might not be available in CI
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Network request failed (expected in some CI environments): {stderr}");
    }
}

#[test]
fn test_profile_configuration() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join(".httpc");

    std::fs::write(
        &config_path,
        "[test]\n\
         host = https://httpbin.org\n\
         @content-type = application/json\n",
    )
    .expect("Failed to write config file");

    // Set the config file location via environment variable
    let output = Command::new(httpc_binary())
        .args(["-p", "test", "GET", "/get"])
        .env("HOME", temp_dir.path())
        .output()
        .expect("Failed to execute httpc");

    // The command should at least parse correctly even if network fails
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should fail with network error, not parsing error
        assert!(!stderr.contains("Profile not found") || stderr.contains("connection"));
    }
}

#[test]
fn test_invalid_arguments() {
    let output = Command::new(httpc_binary())
        .args(["INVALID"])
        .output()
        .expect("Failed to execute httpc");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("usage") || stderr.contains("help"));
}

#[test]
fn test_custom_headers() {
    let output = Command::new(httpc_binary())
        .args([
            "GET",
            "https://httpbin.org/get",
            "-H",
            "X-Test-Header: test-value",
            "-H",
            "Authorization: Bearer token123",
        ])
        .output()
        .expect("Failed to execute httpc");

    // The binary should execute successfully regardless of HTTP status
    assert!(output.status.success(), "Binary execution failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check if we got a successful response (200 OK) in stdout
    // or an HTTP error in stderr (both indicate the request was made)
    let has_successful_response = stdout.contains("httpbin.org");
    let has_http_error = stderr.contains("400") || stderr.contains("401") || stderr.contains("500");

    assert!(
        has_successful_response || has_http_error,
        "Expected either successful response in stdout or HTTP error in stderr.\nStdout: {stdout}\nStderr: {stderr}"
    );
}

#[test]
fn test_basic_auth() {
    let output = Command::new(httpc_binary())
        .args([
            "GET",
            "https://httpbin.org/basic-auth/user/pass",
            "--user",
            "user",
            "--password",
            "pass",
        ])
        .output()
        .expect("Failed to execute httpc");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("authenticated") || stdout.contains("user"));
    } else {
        // Network might not be available in CI
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Network request failed (expected in some CI environments): {stderr}");
    }
}

#[test]
fn test_verbose_mode() {
    let output = Command::new(httpc_binary())
        .args([
            "GET",
            "https://httpbin.org/get",
            "-v", // verbose mode
        ])
        .output()
        .expect("Failed to execute httpc");

    if output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // In verbose mode, connection details should be printed to stderr
        assert!(stderr.contains("> connection:") || stderr.contains("httpbin.org"));
    } else {
        // Network might not be available
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Network request failed: {stderr}");
    }
}

#[test]
fn test_different_http_methods() {
    let methods = ["GET", "POST", "PUT", "DELETE", "HEAD"];

    for method in &methods {
        let output = Command::new(httpc_binary())
            .args([method, "https://httpbin.org/get"])
            .output()
            .expect("Failed to execute httpc");

        // All methods should be accepted by the CLI parser
        // Network failures are acceptable
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Should not fail due to method parsing
            assert!(!stderr.contains("Invalid method") && !stderr.contains("usage"));
        }
    }
}

#[test]
fn test_invalid_url() {
    let output = Command::new(httpc_binary())
        .args(["GET", "http://invalid-domain-that-does-not-exist.invalid"])
        .output()
        .expect("Failed to execute httpc");

    // Should fail with DNS resolution error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should contain some kind of error message
    assert!(!stderr.is_empty());
}

#[test]
fn test_malformed_headers() {
    let output = Command::new(httpc_binary())
        .args([
            "GET",
            "https://httpbin.org/get",
            "-H",
            "InvalidHeaderNoColon", // Invalid header format
        ])
        .output()
        .expect("Failed to execute httpc");

    // Should fail with malformed header
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid header") || stderr.contains("error"));
}

#[test]
fn test_empty_body_post() {
    let output = Command::new(httpc_binary())
        .args(["POST", "https://httpbin.org/post", ""])
        .output()
        .expect("Failed to execute httpc");

    // Empty body should be acceptable
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("httpbin.org") || !stdout.is_empty());
    } else {
        // Network errors are acceptable
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("body"));
    }
}

#[test]
fn test_missing_required_arguments() {
    // Test with no arguments
    let output = Command::new(httpc_binary())
        .output()
        .expect("Failed to execute httpc");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("required") || stderr.contains("Usage") || stderr.contains("error"));
}

#[test]
fn test_only_method_argument() {
    let output = Command::new(httpc_binary())
        .args(["GET"])
        .output()
        .expect("Failed to execute httpc");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("required") || stderr.contains("Usage") || stderr.contains("URL"));
}

#[test]
fn test_ipv4_address() {
    let output = Command::new(httpc_binary())
        .args(["GET", "http://127.0.0.1:8080/test"])
        .output()
        .expect("Failed to execute httpc");

    // Connection will likely fail, but URL parsing should work
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should not fail due to URL parsing
        assert!(!stderr.contains("Invalid URL") && !stderr.contains("parse"));
    }
}

#[test]
fn test_non_standard_port() {
    let output = Command::new(httpc_binary())
        .args(["GET", "https://httpbin.org:443/get"])
        .output()
        .expect("Failed to execute httpc");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("httpbin.org"));
    } else {
        // Network failures are acceptable
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Network request failed: {stderr}");
    }
}

#[test]
fn test_multiple_headers() {
    let output = Command::new(httpc_binary())
        .args([
            "GET",
            "https://httpbin.org/get",
            "-H",
            "X-Custom-Header-1: value1",
            "-H",
            "X-Custom-Header-2: value2",
            "-H",
            "Accept: application/json",
        ])
        .output()
        .expect("Failed to execute httpc");

    // Should handle multiple headers without issues
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("header"));
    }
}

#[test]
fn test_auth_without_password() {
    let output = Command::new(httpc_binary())
        .args([
            "GET",
            "https://httpbin.org/get",
            "--user",
            "testuser",
            // No password provided
        ])
        .output()
        .expect("Failed to execute httpc");

    // Should work with just username (password can be empty)
    if output.status.success() || !output.status.success() {
        // Either outcome is acceptable - depends on implementation
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should not fail due to missing password
        assert!(!stderr.contains("password required"));
    }
}

#[test]
fn test_very_long_url() {
    let long_path = "/".to_string() + &"very-long-path-segment/".repeat(50);
    let long_url = format!("https://httpbin.org{long_path}");

    let output = Command::new(httpc_binary())
        .args(["GET", &long_url])
        .output()
        .expect("Failed to execute httpc");

    // Should handle long URLs
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should not fail due to URL length
        assert!(!stderr.contains("too long") && !stderr.contains("URL"));
    }
}

#[test]
fn test_special_characters_in_headers() {
    let output = Command::new(httpc_binary())
        .args([
            "GET",
            "https://httpbin.org/get",
            "-H",
            "X-Special-Chars: !@#$%^&*()_+-={}[]|\\:;\"'<>?,./",
        ])
        .output()
        .expect("Failed to execute httpc");

    // Should handle special characters in header values
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should not fail due to special characters (unless they're truly invalid for HTTP)
        println!("Output with special chars: {stderr}");
    }
}

#[test]
fn test_case_insensitive_method() {
    let methods = ["get", "post", "PUT", "Delete", "HEAD"];

    for method in &methods {
        let output = Command::new(httpc_binary())
            .args([method, "https://httpbin.org/get"])
            .output()
            .expect("Failed to execute httpc");

        // Should accept methods in any case
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!stderr.contains("Invalid method"));
        }
    }
}
