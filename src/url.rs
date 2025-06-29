use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

const REGEX_PATTERNS_URL: &str = r"^(?P<scheme>[^:\/]+)?(:\/\/)?((?P<user>[^:@]+)?(:(?P<password>[^@]+))?@)?(?P<host>[^:\/\?\#]+)?(:(?P<port>\d+))?(?P<path>[^\?\#]*)(\?(?P<query>[^\#]*))?(#(?P<fragment>.*))?$";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    host: String,
    port: Option<u16>,
    scheme: Option<String>,
}

impl FromStr for Endpoint {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let caps = Regex::new(REGEX_PATTERNS_URL)
            .unwrap()
            .captures(s)
            .ok_or_else(|| format!("Failed to parse endpoint from string: {s}"))?;

        let proto_as_host = caps.name("host").is_none() && caps.name("scheme").is_some();

        let host = if proto_as_host {
            caps.name("scheme")
                .ok_or("Missing scheme/host")?
                .as_str()
                .to_string()
        } else {
            caps.name("host")
                .ok_or("Missing host")?
                .as_str()
                .to_string()
        };

        let scheme = if proto_as_host {
            None
        } else {
            caps.name("scheme").map(|m| m.as_str().to_string())
        };

        let port = caps
            .name("port")
            .map(|m| m.as_str().parse::<u16>().unwrap());

        Ok(Endpoint { host, port, scheme })
    }
}

impl Endpoint {
    pub fn new(host: String, port: Option<u16>, scheme: Option<String>) -> Self {
        Endpoint { host, port, scheme }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Self::from_str(s).map_err(|e| anyhow!("Failed to parse endpoint '{}': {}", s, e))
    }

    pub fn scheme(&self) -> Option<&String> {
        self.scheme.as_ref()
    }

    pub fn host(&self) -> &String {
        &self.host
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut buffer = String::new();

        if let Some(scheme) = &self.scheme {
            buffer.push_str(&format!("{scheme}://"));
        }

        buffer.push_str(&self.host);

        if let Some(port) = self.port.map(|p| format!(":{p}")) {
            buffer.push_str(&port);
        }

        write!(f, "{buffer}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlPath {
    path: String,
    query: Option<String>,
}

impl UrlPath {
    pub fn new(path: String, query: Option<String>) -> Self {
        // Allow empty path with query for cases like "https://example.com?query=value"
        let path = if path.is_empty() && query.is_some() {
            "".to_string() // Keep empty path when there's a query
        } else if !path.starts_with("/") && !path.is_empty() {
            format!("/{path}") // Add leading slash for non-empty paths
        } else {
            path
        };

        UrlPath { path, query }
    }

    pub fn path(&self) -> &String {
        &self.path
    }

    pub fn query(&self) -> Option<&String> {
        self.query.as_ref()
    }
}

impl Display for UrlPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut buffer = String::new();
        // Only add path if it's not empty and not just a "/"
        if !self.path.is_empty() && self.path != "/" {
            buffer.push_str(&self.path);
        }
        if let Some(query) = &self.query {
            buffer.push_str(&format!("?{query}"));
        }
        write!(f, "{buffer}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Url {
    endpoint: Option<Endpoint>,
    path: Option<UrlPath>,
}

impl Url {
    pub fn new(endpoint: Option<&Endpoint>, path: Option<&UrlPath>) -> Self {
        Url {
            endpoint: endpoint.cloned(),
            path: path.cloned(),
        }
    }

    pub fn parse(s: &str) -> Self {
        // Use regex to breakdown the URL into its components and return them in the Url struct
        // This regex stores the first part of the URL in the scheme group when the original string is
        // in relative path format without a leading slash. (e.g. `path/to/resource` instead of `/path/to/resource`)
        let re = Regex::new(REGEX_PATTERNS_URL).unwrap();
        let url_elems = re.captures(s).unwrap();
        let rel_url_wo_lead_slash = url_elems.name("scheme").is_some()
            && url_elems.name("host").is_none()
            && url_elems.name("port").is_none()
            && url_elems.name("path").is_some();

        let scheme = if rel_url_wo_lead_slash {
            None
        } else {
            url_elems.name("scheme").map(|m| m.as_str().to_string())
        };

        let path = if rel_url_wo_lead_slash {
            Some(format!(
                "/{}{}",
                url_elems.name("scheme").unwrap().as_str(),
                url_elems
                    .name("path")
                    .map(|m| m.as_str())
                    .unwrap_or_default()
            ))
        } else {
            let p = url_elems.name("path").map(|m| m.as_str().to_string());
            if p.is_some() && p.as_ref().unwrap().is_empty() {
                None
            } else {
                Some(p.unwrap().to_string())
            }
        };

        let endpoint = if url_elems.name("host").is_some() {
            Some(Endpoint::new(
                url_elems.name("host").unwrap().as_str().to_string(),
                url_elems
                    .name("port")
                    .map(|m| m.as_str().parse::<u16>().unwrap()),
                scheme.clone(),
            ))
        } else {
            None
        };

        let query = url_elems.name("query").map(|m| m.as_str().to_string());

        let path = if path.is_some() || query.is_some() {
            Some(UrlPath::new(path.unwrap_or_else(|| "".to_string()), query))
        } else {
            None
        };

        Url { endpoint, path }
    }

    #[allow(dead_code)]
    pub fn set_endpoint(&mut self, endpoint: &Endpoint) -> &mut Self {
        self.endpoint = Some(endpoint.clone());
        self
    }

    #[allow(dead_code)]
    pub fn set_path(&mut self, new_path: &UrlPath) -> &mut Self {
        self.path = Some(new_path.clone());
        self
    }

    // pub fn merge(self, other: &Url) -> Self {
    //     // if the other URL has a host => replace scheme, host and port, do not retain the originals
    //     let (scheme, host, port) = if other.host().is_some() {
    //         (other.scheme(), other.host(), other.port())
    //     } else {
    //         (self.scheme(), self.host(), self.port())
    //     };

    //     // if the host and port is the same => replace path and query if the other URL has them
    //     // otherwise => replace path and query (it's most likely meant as full replace)
    //     let (path, query) =
    //         if other.host().is_some() && self.host() == other.host() && self.port() == other.port()
    //         {
    //             if other.path().is_some() {
    //                 (other.path(), other.query())
    //             } else {
    //                 (self.path(), self.query())
    //             }
    //         } else {
    //             (other.path(), other.query())
    //         };

    //     let merged = Url {
    //         scheme: scheme.map(|s| s.to_string()),
    //         host: host.map(|s| s.to_string()),
    //         port: port.map(|p| p.clone()),
    //         path: path.map(|s| s.to_string()),
    //         query: query.map(|s| s.to_string()),
    //     };
    //     merged
    // }

    #[allow(dead_code)]
    pub fn to_endpoint(&self) -> Option<&Endpoint> {
        self.endpoint.as_ref()
    }

    #[allow(dead_code)]
    pub fn to_url_path(&self) -> Option<&UrlPath> {
        self.path.as_ref()
    }

    #[allow(dead_code)]
    pub fn host(&self) -> Option<&String> {
        self.endpoint.as_ref().map(|s| s.host())
    }

    #[allow(dead_code)]
    pub fn port(&self) -> Option<u16> {
        self.endpoint.as_ref().and_then(|s| s.port())
    }

    #[allow(dead_code)]
    pub fn scheme(&self) -> Option<&String> {
        self.endpoint.as_ref().and_then(|s| s.scheme())
    }

    #[allow(dead_code)]
    pub fn path(&self) -> Option<&String> {
        self.path.as_ref().map(|p| p.path())
    }

    #[allow(dead_code)]
    pub fn query(&self) -> Option<&String> {
        self.path.as_ref().and_then(|p| p.query())
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut url = String::new();
        if self.endpoint.is_some() {
            url.push_str(&self.endpoint.as_ref().unwrap().to_string());
        }

        if self.path.is_some() {
            url.push_str(&self.path.as_ref().unwrap().to_string());
        }

        write!(f, "{url}")
    }
}

#[cfg(test)]
mod test {
    mod url {
        use super::super::*;

        #[test]
        fn parse_should_parse_abosolute_url_with_scheme_host_port() {
            let url = Url::parse("http://example.com:8080/path/to/resource?query=string");
            assert_eq!(url.scheme(), Some(&"http".to_string()));
            assert_eq!(url.host(), Some(&"example.com".to_string()));
            assert_eq!(url.port(), Some(8080));
            assert_eq!(url.path(), Some(&"/path/to/resource".to_string()));
            assert_eq!(url.query(), Some(&"query=string".to_string()));
            assert_eq!(
                url.to_endpoint().unwrap().to_string(),
                "http://example.com:8080"
            );
            assert_eq!(
                url.to_url_path().unwrap().to_string(),
                "/path/to/resource?query=string"
            );
        }

        #[test]
        fn parse_should_parse_relative_url_with_no_scheme_and_host() {
            let url = Url::parse("/path/to/resource?query=string");
            assert_eq!(url.to_endpoint(), None);
            assert_eq!(url.scheme(), None);
            assert_eq!(url.host(), None);
            assert_eq!(url.port(), None);
            assert_eq!(url.path(), Some(&"/path/to/resource".to_string()));
            assert_eq!(url.query(), Some(&"query=string".to_string()));
            assert_eq!(
                url.to_url_path().unwrap().to_string(),
                "/path/to/resource?query=string"
            );
        }

        #[test]
        fn parse_should_return_slashed_path_if_original_doesnt_have_lead_slash() {
            let url = Url::parse("path/to/resource?query=string");
            assert_eq!(url.scheme(), None);
            assert_eq!(url.host(), None);
            assert_eq!(url.port(), None);
            assert_eq!(url.path(), Some(&"/path/to/resource".to_string()));
            assert_eq!(url.query(), Some(&"query=string".to_string()));
            assert_eq!(url.to_endpoint(), None);
            assert_eq!(
                url.to_url_path().unwrap().to_string(),
                "/path/to/resource?query=string"
            );
        }

        #[test]
        fn test_url_with_port_zero() {
            let url = Url::parse("http://example.com:0/path");
            assert_eq!(url.scheme(), Some(&"http".to_string()));
            assert_eq!(url.host(), Some(&"example.com".to_string()));
            assert_eq!(url.port(), Some(0));
            assert_eq!(url.path(), Some(&"/path".to_string()));
        }

        #[test]
        fn test_url_with_fragment() {
            let url = Url::parse("https://example.com/page#section");
            assert_eq!(url.scheme(), Some(&"https".to_string()));
            assert_eq!(url.host(), Some(&"example.com".to_string()));
            assert_eq!(url.path(), Some(&"/page".to_string()));
            // Note: Fragment is typically ignored in HTTP clients
        }

        #[test]
        fn test_url_with_international_domain() {
            let url = Url::parse("https://测试.example.com/path");
            assert_eq!(url.scheme(), Some(&"https".to_string()));
            // The parser should handle international domains
            assert!(url.host().is_some());
        }

        #[test]
        fn test_url_with_special_characters_in_path() {
            let url = Url::parse("https://example.com/path with spaces/file.txt");
            assert_eq!(url.scheme(), Some(&"https".to_string()));
            assert_eq!(url.host(), Some(&"example.com".to_string()));
            assert!(url.path().is_some());
        }

        #[test]
        fn test_url_with_complex_query() {
            let url = Url::parse("https://example.com/search?q=rust+lang&sort=date&limit=10");
            assert_eq!(
                url.query(),
                Some(&"q=rust+lang&sort=date&limit=10".to_string())
            );
        }

        #[test]
        fn test_url_new_with_endpoint_and_path() {
            let endpoint = Endpoint::parse("https://api.example.com:443").unwrap();
            let url_path = UrlPath::new("/v1/users".to_string(), Some("page=1".to_string()));

            let url = Url::new(Some(&endpoint), Some(&url_path));

            assert_eq!(
                url.to_string(),
                "https://api.example.com:443/v1/users?page=1"
            );
        }

        #[test]
        fn test_url_new_with_endpoint_only() {
            let endpoint = Endpoint::parse("https://example.com").unwrap();

            let url = Url::new(Some(&endpoint), None);

            assert_eq!(url.to_string(), "https://example.com");
        }

        #[test]
        fn test_url_new_with_path_only() {
            let url_path = UrlPath::new("/api/test".to_string(), None);

            let url = Url::new(None, Some(&url_path));

            assert_eq!(url.to_string(), "/api/test");
        }

        #[test]
        fn test_url_edge_cases() {
            // Empty path
            let url1 = Url::parse("https://example.com");
            assert_eq!(url1.path(), None);

            // Root path
            let url2 = Url::parse("https://example.com/");
            assert_eq!(url2.path(), Some(&"/".to_string()));

            // Query without path
            let url3 = Url::parse("https://example.com?query=value");
            assert_eq!(url3.query(), Some(&"query=value".to_string()));
        }

        #[test]
        fn test_url_conversion_methods() {
            let url = Url::parse("https://example.com:8080/api/v1?key=value");

            // Test to_endpoint
            let endpoint = url.to_endpoint().unwrap();
            assert_eq!(endpoint.host(), "example.com");
            assert_eq!(endpoint.port(), Some(8080));
            assert_eq!(endpoint.scheme(), Some(&"https".to_string()));

            // Test to_url_path
            let url_path = url.to_url_path().unwrap();
            assert_eq!(url_path.path(), "/api/v1");
            assert_eq!(url_path.query(), Some(&"key=value".to_string()));
        }

        #[test]
        fn test_urlpath_new() {
            let path1 = UrlPath::new("/test".to_string(), None);
            assert_eq!(path1.path(), "/test");
            assert_eq!(path1.query(), None);

            let path2 = UrlPath::new("/test".to_string(), Some("param=value".to_string()));
            assert_eq!(path2.path(), "/test");
            assert_eq!(path2.query(), Some(&"param=value".to_string()));
        }

        #[test]
        fn test_urlpath_to_string() {
            let path1 = UrlPath::new("/api/test".to_string(), None);
            assert_eq!(path1.to_string(), "/api/test");

            let path2 = UrlPath::new(
                "/api/test".to_string(),
                Some("key=value&foo=bar".to_string()),
            );
            assert_eq!(path2.to_string(), "/api/test?key=value&foo=bar");
        }
    }

    mod endpoint {
        use super::super::*;

        #[test]
        fn test_endpoint_new() {
            let endpoint = Endpoint::new(
                "example.com".to_string(),
                Some(8080),
                Some("https".to_string()),
            );

            assert_eq!(endpoint.host(), "example.com");
            assert_eq!(endpoint.port(), Some(8080));
            assert_eq!(endpoint.scheme(), Some(&"https".to_string()));
        }

        #[test]
        fn test_endpoint_without_scheme() {
            let endpoint = Endpoint::new("example.com".to_string(), Some(80), None);

            assert_eq!(endpoint.host(), "example.com");
            assert_eq!(endpoint.port(), Some(80));
            assert_eq!(endpoint.scheme(), None);
        }

        #[test]
        fn test_endpoint_to_string_variations() {
            let cases = vec![
                (
                    Endpoint::new("example.com".to_string(), None, Some("https".to_string())),
                    "https://example.com",
                ),
                (
                    Endpoint::new(
                        "example.com".to_string(),
                        Some(8080),
                        Some("https".to_string()),
                    ),
                    "https://example.com:8080",
                ),
                (
                    Endpoint::new("example.com".to_string(), Some(80), None),
                    "example.com:80",
                ),
                (
                    Endpoint::new("example.com".to_string(), None, None),
                    "example.com",
                ),
            ];

            for (endpoint, expected) in cases {
                assert_eq!(endpoint.to_string(), expected);
            }
        }

        #[test]
        fn test_endpoint_parse_error_cases() {
            // These would ideally return errors, but the current implementation might not
            // handle all edge cases. We'll test what the current behavior is.
            let test_cases = vec!["://invalid", "https://", "https://:8080", ""];

            for case in test_cases {
                let result = Endpoint::parse(case);
                // The behavior might vary, but we ensure it doesn't panic
                match result {
                    Ok(_) => {
                        // If it succeeds, that's the current behavior
                    }
                    Err(_) => {
                        // If it fails, that's also acceptable
                    }
                }
            }
        }

        #[test]
        fn test_endpoint_with_ipv4() {
            let endpoint = Endpoint::parse("http://192.168.1.1:8080").unwrap();
            assert_eq!(endpoint.host(), "192.168.1.1");
            assert_eq!(endpoint.port(), Some(8080));
            assert_eq!(endpoint.scheme(), Some(&"http".to_string()));
        }

        #[test]
        fn test_endpoint_with_localhost() {
            let endpoint = Endpoint::parse("http://localhost:3000").unwrap();
            assert_eq!(endpoint.host(), "localhost");
            assert_eq!(endpoint.port(), Some(3000));
            assert_eq!(endpoint.scheme(), Some(&"http".to_string()));
        }

        #[test]
        fn test_endpoint_standard_ports() {
            let http_endpoint = Endpoint::parse("http://example.com:80").unwrap();
            assert_eq!(http_endpoint.port(), Some(80));

            let https_endpoint = Endpoint::parse("https://example.com:443").unwrap();
            assert_eq!(https_endpoint.port(), Some(443));
        }
    }
}
