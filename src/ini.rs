use crate::http::HttpConnectionProfile;
use crate::stdio::{ask, ask_binary, ask_no_space_string, ask_path};
use crate::url::Endpoint;
use crate::utils::Result;

use ini::{Ini, Properties};
use std::collections::HashMap;

pub const DEFAULT_INI_FILE_PATH: &str = "~/.wiq";
pub const DEFAULT_SCHEME: &str = "http";
pub const PROFILE_BLANK: &str = "none";

const INI_HOST: &str = "host";
const INI_PORT: &str = "port";
const INI_SCHEME: &str = "scheme";
const INI_USER: &str = "user";
const INI_PASSWORD: &str = "password";
const INI_CA_CERT: &str = "ca_cert";
const INI_INSECURE: &str = "insecure";
const INI_PROXY: &str = "proxy";

#[derive(Debug)]
pub struct IniProfile {
    name: String,
    server: Option<Endpoint>,
    user: Option<String>,
    password: Option<String>,
    insecure: Option<bool>,
    ca_cert: Option<String>,
    headers: HashMap<String, String>,
    proxy: Option<Endpoint>,
}

impl HttpConnectionProfile for IniProfile {
    fn server(&self) -> Option<&Endpoint> {
        self.server.as_ref()
    }

    fn user(&self) -> Option<&String> {
        self.user.as_ref()
    }

    fn password(&self) -> Option<&String> {
        self.password.as_ref()
    }

    fn insecure(&self) -> Option<bool> {
        self.insecure
    }

    fn ca_cert(&self) -> Option<&String> {
        self.ca_cert.as_ref()
    }

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    fn proxy(&self) -> Option<&Endpoint> {
        self.proxy.as_ref()
    }
}

impl IniProfile {
    pub fn merge_profile<T>(&mut self, other: &T) -> &mut Self
    where
        // The reason using Generic is to force Debug trait to
        // be implemented for testing purpose.
        // We can revert it to `impl HttpConnectionProfile` if we
        // don't need to test it.
        T: HttpConnectionProfile + std::fmt::Debug,
    {
        if other.server().is_some() {
            self.server = other.server().cloned();
        }
        if other.user().is_some() {
            self.user = other.user().cloned();
            self.password = other.password().cloned();
        }
        if other.insecure().is_some() {
            self.insecure = other.insecure();
        }
        if other.ca_cert().is_some() {
            self.ca_cert = other.ca_cert().cloned();
        }
        if other.headers().len() > 0 {
            for (k, v) in other.headers() {
                self.headers.insert(k.clone(), v.clone());
            }
        }
        if other.proxy().is_some() {
            self.proxy = other.proxy().cloned();
        }

        self
    }
}

pub struct IniProfileStore {
    file_path: String,
}

impl IniProfileStore {
    pub fn new(file_path: &str) -> Self {
        let file_path = shellexpand::tilde(file_path).to_string();
        Self { file_path }
    }

    pub fn get_profile(&self, name: &str) -> Result<Option<IniProfile>> {
        if name == PROFILE_BLANK {
            return Ok(Some(get_blank_profile()));
        }

        let ini = if std::path::Path::new(&self.file_path).exists() {
            Ini::load_from_file(&self.file_path)?
        } else {
            return Ok(None);
        };

        let section = match ini.section(Some(name.to_string())) {
            Some(s) => s,
            None => {
                return Ok(None);
            }
        };

        let mut headers = HashMap::<String, String>::new();
        for (key, value) in section.iter() {
            // here, we'll pick up only ones start with at sign
            if let Some(stripped) = key.strip_prefix("@") {
                headers.insert(stripped.to_string().to_lowercase(), value.to_string());
            }
        }

        fn try_get<T>(section: &Properties, key: &str) -> Option<T>
        where
            T: std::str::FromStr,
            T::Err: std::fmt::Debug,
        {
            section.get(key).map(|s| s.parse::<T>().unwrap())
        }

        let host = try_get::<String>(&section, INI_HOST);

        if host.is_none() {
            return Err(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Missing 'host' entity in '{}' profile", name),
                )
                .into(),
            );
        }

        let port = try_get::<u16>(&section, INI_PORT);
        let scheme = if let Some(scheme) = try_get::<String>(&section, INI_SCHEME) {
            Some(scheme)
        } else {
            Some(DEFAULT_SCHEME.to_string())
        };
        //    try_get::<String>(&section, INI_SCHEME).or(Some(DEFAULT_SCHEME.to_string()));

        let endpoint = if let Some(host) = host {
            Some(Endpoint::new(host, port, scheme))
        } else {
            None
        };

        let profile = IniProfile {
            name: name.to_string(),
            server: endpoint,
            user: try_get(&section, INI_USER),
            password: try_get(&section, INI_PASSWORD),
            insecure: try_get::<bool>(&section, INI_INSECURE),
            ca_cert: try_get(&section, INI_CA_CERT),
            headers: headers.clone(),
            proxy: try_get::<Endpoint>(&section, INI_PROXY),
        };

        Ok(Some(profile))
    }

    #[allow(dead_code)]
    pub fn put_profile(&self, profile: &IniProfile) -> Result<()> {
        let mut ini = Ini::new();
        let mut section = ini.with_section(Some(profile.name.clone()));

        if profile.server().is_some() {
            let endpoint = profile.server().unwrap();
            section.set(INI_HOST, endpoint.host().to_string());
            if endpoint.port().is_some() {
                section.set(INI_PORT, endpoint.port().unwrap().to_string());
            }
            section.set(INI_SCHEME, endpoint.scheme().unwrap().to_string());
        }
        if profile.user().is_some() {
            section.set(INI_USER, profile.user().unwrap());
        }
        if profile.password().is_some() {
            section.set(INI_PASSWORD, profile.password().unwrap());
        }
        section.set(INI_INSECURE, profile.insecure().unwrap().to_string());

        if profile.ca_cert().is_some() {
            section.set(INI_CA_CERT, profile.ca_cert().unwrap());
        }

        for (k, v) in profile.headers.iter() {
            section.set(format!("@{}", k), v);
        }

        ini.write_to_file(&self.file_path).unwrap();

        Ok(())
    }
}

pub fn get_blank_profile() -> IniProfile {
    IniProfile {
        name: PROFILE_BLANK.to_string(),
        server: None,
        user: None,
        password: None,
        insecure: None,
        ca_cert: None,
        headers: HashMap::new(),
        proxy: None
    }
}

#[allow(dead_code)]
pub fn ask_new_profile(name: &str, i: &std::io::Stdin) -> Result<Option<IniProfile>> {
    let init_msg = format!(
        "Profile \"{}\" doesn't exist. Do you want to create it? [y/N]: ",
        name
    );
    if !ask_binary(&i, &init_msg)? {
        return Ok(None);
    }

    let host = ask_no_space_string(&i, "host name: ")?;
    let port = ask::<String>(&i, "port: ", r"\d+")?;
    let scheme = if ask_binary(&i, "use SSL/TLS? [y/N]: ")? {
        "https"
    } else {
        "http"
    };
    let user = if ask_binary(&i, "Do you need a user/password for this URL? [y/N]: ")? {
        Some(ask_no_space_string(&i, "user: ")?)
    } else {
        None
    };

    let password = if user.is_some() {
        Some(ask_no_space_string(&i, "password: ")?)
    } else {
        None
    };

    let ca_cert = if scheme == "https"
        && ask_binary(&i, "Do you need to use a custom CA certificate? [y/N]: ")?
    {
        let path = ask_path(&i, "CA certificate file: ")?;
        Some(path)
    } else {
        None
    };

    Ok(Some(IniProfile {
        name: name.to_string(),
        server: Some(Endpoint::new(
            host,
            Some(port.parse::<u16>().unwrap()),
            Some(scheme.to_string()),
        )),
        user: user,
        password: password,
        insecure: Some(false),
        ca_cert: ca_cert,
        headers: HashMap::new(),
        proxy: None
    }))
}
#[cfg(test)]
mod test {
    use super::*;
    use std::fs::remove_file;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempPath};

    const TEST_HOST: &str = "test-server";
    const TEST_PORT: &str = "8082";
    const TEST_SCHEME: &str = "http";
    const TEST_USER: &str = "test_user";
    const TEST_PASSWORD: &str = "test_password";
    const TEST_CA_CERT: &str = "/etc/pki/ca/cert.crt";
    const TEST_CONTENT_TYPE: &str = "application/json";
    const TEST_INSECURE: bool = false;
    const TEST_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
    const DEFAULT_INI_SECTION: &str = "default";

    fn create_ini_file() -> Result<TempPath> {
        let content = format!(
            "[{}]\n\
             host={}\n\
             port={}\n\
             scheme={}\n\
             user={}\n\
             password={}\n\
             ca_cert={}\n\
             insecure=false\n\
             @Content-Type={}\n\
             @User-Agent={}\n\
             ",
            DEFAULT_INI_SECTION,
            TEST_HOST,
            TEST_PORT,
            TEST_SCHEME,
            TEST_USER,
            TEST_PASSWORD,
            TEST_CA_CERT,
            TEST_CONTENT_TYPE,
            TEST_USER_AGENT
        );

        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;
        let path = file.into_temp_path();
        Ok(path)
    }

    fn delete_ini_file(file_path: &str) -> Result<()> {
        let path = shellexpand::tilde(file_path).to_string();
        if std::path::Path::new(&path).exists() {
            remove_file(path)?;
        }
        Ok(())
    }

    fn test_profile(path: &str) -> Result<IniProfile> {
        let profile = IniProfileStore::new(&path)
            .get_profile(DEFAULT_INI_SECTION)?
            .unwrap();
        assert!(profile.server().is_some());
        let endpoint = profile.server().unwrap();
        assert_eq!(endpoint.host(), &TEST_HOST.to_string());
        assert_eq!(endpoint.port(), Some(TEST_PORT.parse::<u16>().unwrap()));
        assert_eq!(endpoint.scheme(), Some(&TEST_SCHEME.to_string()));
        assert_eq!(profile.user(), Some(&TEST_USER.to_string()));
        assert_eq!(profile.password(), Some(&TEST_PASSWORD.to_string()));
        assert_eq!(profile.ca_cert(), Some(&TEST_CA_CERT.to_string()));
        assert_eq!(profile.insecure(), Some(TEST_INSECURE));

        assert_eq!(profile.headers.len(), 2);
        assert_eq!(
            profile.headers["content-type"],
            TEST_CONTENT_TYPE.to_string()
        );
        assert_eq!(profile.headers["user-agent"], TEST_USER_AGENT.to_string());

        Ok(profile)
    }

    #[test]
    fn load_profile_should_return_correct_values_in_ini_file() -> Result<()> {
        let temp_path = create_ini_file()?;
        let path = temp_path.as_os_str().to_str().unwrap().to_string();
        let _ = test_profile(&path)?;
        temp_path.close()?;
        Ok(())
    }

    #[test]
    fn add_profile_should_properly_add_record_in_ini_file() -> Result<()> {
        let endpoint = Endpoint::new(
            TEST_HOST.to_string(),
            Some(TEST_PORT.parse::<u16>().unwrap()),
            Some(TEST_SCHEME.to_string()),
        );

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), TEST_CONTENT_TYPE.to_string());
        headers.insert("User-Agent".to_string(), TEST_USER_AGENT.to_string());

        let profile = IniProfile {
            name: DEFAULT_INI_SECTION.to_string(),
            server: Some(endpoint),
            user: Some(TEST_USER.to_string()),
            password: Some(TEST_PASSWORD.to_string()),
            insecure: Some(TEST_INSECURE),
            ca_cert: Some(TEST_CA_CERT.to_string()),
            headers: headers,
            proxy: None
        };

        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap().to_string();

        let _ = IniProfileStore::new(&path).put_profile(&profile)?;
        let _ = test_profile(&path)?;
        let _ = delete_ini_file(&path)?;

        Ok(())
    }

    #[derive(Debug)]
    struct TestArgs {
        url: Endpoint,
        user: String,
        password: String,
        ca_cert: String,
        headers: HashMap<String, String>,
        proxy: Option<Endpoint>,
    }

    impl TestArgs {
        fn new(
            url: &Endpoint,
            user: &str,
            password: &str,
            ca_cert: &str,
            headers: &HashMap<String, String>,
        ) -> Self {
            Self {
                url: url.clone(),
                user: user.to_string(),
                password: password.to_string(),
                ca_cert: ca_cert.to_string(),
                headers: headers.clone(),
                proxy: None,
            }
        }
    }

    impl HttpConnectionProfile for TestArgs {
        fn server(&self) -> Option<&Endpoint> {
            Some(&self.url)
        }

        fn user(&self) -> Option<&String> {
            Some(&self.user)
        }

        fn password(&self) -> Option<&String> {
            Some(&self.password)
        }

        fn insecure(&self) -> Option<bool> {
            Some(false)
        }

        fn ca_cert(&self) -> Option<&String> {
            Some(&self.ca_cert)
        }

        fn headers(&self) -> &HashMap<String, String> {
            &self.headers
        }

        fn proxy(&self) -> Option<&Endpoint> {
            self.proxy.as_ref()
        }
    }

    #[test]
    fn ini_profile_merge_should_merge_req_members_properly() -> Result<()> {
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        headers.insert("user-agent".to_string(), "Mozilla/5.0".to_string());

        let mut original = IniProfile {
            name: DEFAULT_INI_SECTION.to_string(),
            server: Some(Endpoint::parse(&"https://localhost:8081")),
            user: None,
            password: None,
            insecure: Some(TEST_INSECURE),
            ca_cert: None,
            headers: headers.clone(),
            proxy: None
        };

        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("content-type".to_string(), "text/html".to_string());

        let merging = TestArgs::new(
            &Endpoint::parse(&"http://example.com"),
            "test_user",
            "test_password",
            "/etc/pki/ca/cert.crt",
            &headers,
        );

        let merged = original.merge_profile(&merging);
        let merged_endpoint = merged.server().unwrap();

        assert_eq!(merged_endpoint.to_string(), "http://example.com");
        assert_eq!(merged.user(), Some(&"test_user".to_string()));
        assert_eq!(merged.password(), Some(&"test_password".to_string()));
        assert_eq!(merged.insecure(), Some(TEST_INSECURE));
        assert_eq!(merged.ca_cert(), Some(&"/etc/pki/ca/cert.crt".to_string()));
        assert_eq!(merged.headers.len(), 2);
        assert_eq!(merged.headers["content-type"], "text/html".to_string());
        assert_eq!(merged.headers["user-agent"], "Mozilla/5.0".to_string());

        Ok(())
    }
}
