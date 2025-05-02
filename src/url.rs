use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

const REGEX_PATTERNS_URL: &str = r"^(?P<scheme>[^:\/]+)?(:\/\/)?(?P<host>[^:\/\?]+)?(:(?P<port>\d+))?(?P<path>[^\?]*)(\?(?P<query>.*))?$";

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
            .unwrap_or_else(|| {
                panic!("Failed to parse endpoint from string: {}", s);
            });

        let proto_as_host = caps.name("host").is_none() && caps.name("scheme").is_some();

        let host = if proto_as_host {
            caps.name("scheme").unwrap().as_str().to_string()
        } else {
            caps.name("host").unwrap().as_str().to_string()
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

    pub fn parse(s: &str) -> Self {
        match Self::from_str(s) {
            Ok(endpoint) => endpoint,
            Err(_) => panic!("Failed to parse endpoint from string: {}", s),
        }
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
        let mut buffer = if let Some(scheme) = &self.scheme {
            format!("{}://", scheme)
        } else {
            "plaintext://".to_string()
        };

        buffer.push_str(&self.host);

        if let Some(port) = self.port.map(|p| format!(":{}", p)) {
            buffer.push_str(&port);
        }

        write!(f, "{}", buffer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlPath {
    path: String,
    query: Option<String>,
}

impl UrlPath {
    pub fn new(path: String, query: Option<String>) -> Self {
        if path.is_empty() && query.is_some() {
            panic!("Path cannot be empty if query is provided");
        }
        if !path.starts_with("/") {
            UrlPath {
                path: format!("/{}", path),
                query,
            }
        } else {
            UrlPath { path, query }
        }
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
        let mut buffer = self.path.clone();
        if let Some(query) = &self.query {
            buffer.push_str(&format!("?{}", query));
        }
        write!(f, "{}", buffer)
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
        let caps = re.captures(s).unwrap();
        let rel_url_wo_lead_slash = caps.name("scheme").is_some()
            && caps.name("host").is_none()
            && caps.name("port").is_none()
            && caps.name("path").is_some();

        let scheme = if rel_url_wo_lead_slash {
            None
        } else {
            caps.name("scheme").map(|m| m.as_str().to_string())
        };

        let path = if rel_url_wo_lead_slash {
            Some(format!(
                "/{}{}",
                caps.name("scheme").unwrap().as_str(),
                caps.name("path").map(|m| m.as_str()).unwrap_or_default()
            ))
        } else {
            let p = caps.name("path").map(|m| m.as_str().to_string());
            if p.is_some() && p.as_ref().unwrap().is_empty() {
                None
            } else {
                Some(p.unwrap().to_string())
            }
        };

        let endpoint = if caps.name("host").is_some() {
            Some(Endpoint::new(
                caps.name("host").unwrap().as_str().to_string(),
                caps.name("port")
                    .map(|m| m.as_str().parse::<u16>().unwrap()),
                scheme.clone(),
            ))
        } else {
            None
        };

        let path = if path.is_some() {
            Some(UrlPath::new(
                path.unwrap(),
                caps.name("query").map(|m| m.as_str().to_string()),
            ))
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

        write!(f, "{}", url)
    }
}

#[cfg(test)]
mod test {
    mod endpoint {
        use super::super::*;

        #[test]
        fn parse_should_parse_endpoint_with_scheme_host_and_port() -> crate::utils::Result<()> {
            let endpoint = Endpoint::parse("http://example.com:8080");
            assert_eq!(endpoint.scheme(), Some(&"http".to_string()));
            assert_eq!(endpoint.host(), "example.com");
            assert_eq!(endpoint.port(), Some(8080));
            Ok(())
        }

        #[test]
        fn parse_should_parse_endpoint_without_port() -> crate::utils::Result<()> {
            let endpoint = Endpoint::parse("http://example.com");
            assert_eq!(endpoint.scheme(), Some(&"http".to_string()));
            assert_eq!(endpoint.host(), "example.com");
            assert_eq!(endpoint.port(), None);
            Ok(())
        }

        #[test]
        fn parse_should_parse_endpoint_with_host_only() -> crate::utils::Result<()> {
            let endpoint = Endpoint::parse("example.com");
            assert_eq!(endpoint.scheme(), None);
            assert_eq!(endpoint.host(), "example.com");
            assert_eq!(endpoint.port(), None);
            Ok(())
        }
    }
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

        // #[test]
        // fn merge_should_preserve_host_when_merging_url_is_relative() {
        //     let url1 = Url::parse("https://example.com:9999");
        //     let url2 = Url::parse("path/to/resource?query=string");
        //     let url = url1.merge(&url2);
        //     assert_eq!(url.scheme, Some("https".to_string()));
        //     assert_eq!(url.host, Some("example.com".to_string()));
        //     assert_eq!(url.port, Some(9999));
        //     assert_eq!(url.path, Some("/path/to/resource".to_string()));
        //     assert_eq!(url.query, Some("query=string".to_string()));
        // }

        // #[test]
        // fn merge_should_replace_all_when_merging_url_has_a_host() {
        //     let url1 = Url::parse("https://example.com:9999/path/to/resource?query=string");
        //     let url2 = Url::parse("http://somethingelse.com");
        //     let url = url1.merge(&url2);
        //     assert_eq!(url.scheme, Some("http".to_string()));
        //     assert_eq!(url.host, Some("somethingelse.com".to_string()));
        //     assert_eq!(url.port, None);
        //     assert_eq!(url.path, None);
        //     assert_eq!(url.query, None);
        // }
    }
}
