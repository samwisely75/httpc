# Enhancement Requests (Ordered by Priority)

====================

1. Have `default` profile to fill-in the gap

    - Instead of repeating the same profile configuration in multiple places, use a `default` profile that can be used as a fallback. Current implementation ignores the `default` profile if a user-defined profile exists.
    - We will read the ini twice, once for the `default` profile and once for the user-defined profile if specified.
    - We can use the `merge_profile()` to stack the profiles together, allowing the user-defined profile to override the `default` profile settings.
    - Update the README to reflect the new `default` profile feature and how it can be used.

1. Improve Error Handling:
   - Enhance error messages in `ini.rs` to provide more context, especially in tests.
   - Use `anyhow::Result` for better error handling across the codebase.
   - Stop using `Box<dyn Error>` or `std::error::Error` or `std::fmt::Error` in favor of `anyhow::Error` for more ergonomic error handling.

1. Refactor URL Parsing:

    - Simplify the URL parsing logic in `url.rs` by breaking it down into smaller functions.
    - Use `url::Url` crate for robust URL handling instead of custom regex parsing.

1. Optimize Imports:

    - Remove unused imports in `stdio.rs` and `url.rs`.
    - Use `use std::io::{self, Read};` instead of importing `Read` separately.
    - Consolidate imports from the same module to reduce clutter.
    - Check for unused imports in all files and remove them to keep the code clean.
    - Check for unused crate dependencies in `Cargo.toml` and remove them to reduce build time and binary size.

1. Improve Code Readability:

    - Use more descriptive variable names in `http.rs` to clarify their purpose.
    - Add comments to complex logic in `url.rs` to explain the reasoning behind certain decisions.
    - Use consistent formatting across all files, ensuring proper indentation and spacing.
    - Refactor long functions into smaller, more manageable ones for better readability.

1. Remove cargo fmt warnings:

    - Remove all warninngs from `cargo fmt`.
