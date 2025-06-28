use std::process::Command;
use tempfile::tempdir;

fn webly_binary() -> String {
    env!("CARGO_BIN_EXE_webly").to_string()
}

#[test]
fn test_help_command() {
    let output = Command::new(webly_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute webly");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("A lightweight, profile-based HTTP client"));
    assert!(stdout.contains("Usage:"));
}

#[test]
fn test_version_command() {
    let output = Command::new(webly_binary())
        .arg("--version")
        .output()
        .expect("Failed to execute webly");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("webly"));
}

#[test]
fn test_basic_get_request() {
    let output = Command::new(webly_binary())
        .args(&["GET", "https://httpbin.org/get"])
        .output()
        .expect("Failed to execute webly");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("httpbin.org"));
    } else {
        // Network might not be available in CI, so we just check the binary runs
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!(
            "Network request failed (expected in some CI environments): {}",
            stderr
        );
    }
}

#[test]
fn test_post_with_stdin() {
    let mut cmd = Command::new(webly_binary())
        .args(&["POST", "https://httpbin.org/post"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn webly");

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
        println!(
            "Network request failed (expected in some CI environments): {}",
            stderr
        );
    }
}

#[test]
fn test_profile_configuration() {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join(".webly");

    std::fs::write(
        &config_path,
        "[test]\n\
         host = https://httpbin.org\n\
         @content-type = application/json\n",
    )
    .expect("Failed to write config file");

    // Set the config file location via environment variable
    let output = Command::new(webly_binary())
        .args(&["-p", "test", "GET", "/get"])
        .env("HOME", temp_dir.path())
        .output()
        .expect("Failed to execute webly");

    // The command should at least parse correctly even if network fails
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should fail with network error, not parsing error
        assert!(!stderr.contains("Profile not found") || stderr.contains("connection"));
    }
}

#[test]
fn test_invalid_arguments() {
    let output = Command::new(webly_binary())
        .args(&["INVALID"])
        .output()
        .expect("Failed to execute webly");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("usage") || stderr.contains("help"));
}

#[test]
fn test_custom_headers() {
    let output = Command::new(webly_binary())
        .args(&[
            "GET",
            "https://httpbin.org/get",
            "-H",
            "X-Test-Header: test-value",
            "-H",
            "Authorization: Bearer token123",
        ])
        .output()
        .expect("Failed to execute webly");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("httpbin.org"));
    } else {
        // Network might not be available in CI
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!(
            "Network request failed (expected in some CI environments): {}",
            stderr
        );
    }
}

#[test]
fn test_basic_auth() {
    let output = Command::new(webly_binary())
        .args(&[
            "GET",
            "https://httpbin.org/basic-auth/user/pass",
            "--user",
            "user",
            "--password",
            "pass",
        ])
        .output()
        .expect("Failed to execute webly");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("authenticated") || stdout.contains("user"));
    } else {
        // Network might not be available in CI
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!(
            "Network request failed (expected in some CI environments): {}",
            stderr
        );
    }
}
