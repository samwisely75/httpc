---
applyTo: '**'
---
Provide project context and coding guidelines that AI should follow when generating code, answering questions, or reviewing changes.

# Project Context

This project is a Rust-based HTTP client named `httpc`, previously known as `webly`. It is designed to be lightweight and profile-based, allowing users to manage HTTP requests easily. The project has been recently renamed and restructured, with a focus on improving usability and functionality.

# Coding Guidelines

1. **Code Structure**: Organize code into modules and packages logically. Follow Rust's conventions for module naming and organization.

2. **Error Handling**: Use Rust's `Result` and `Option` types for error handling. Propagate errors using the `?` operator. Use anyhow as a standard error type for convenience.

3. **Testing**: Write unit tests for all public functions. Use the `#[cfg(test)]` module for test cases.

4. **Security**: Follow best practices for secure coding, especially when handling user input and making network requests.

5. **Dependencies**: Keep dependencies up to date and minimize the use of external crates where possible.

6. **Code Reviews**: Participate in code reviews and provide constructive feedback to peers.

7. **String Formatting**: Use embedded expressions for string formatting, e.g., `format!("Hello, {name}")` instead of `format!("Hello, {}", name)`. The latter is deprecated in Rust 2021 edition.

8. **Continuous Integration**: Use CI/CD pipelines to automate testing and deployment processes.

9. Run cargo clippy and cargo fmt before submitting pull requests to ensure code quality and consistency.