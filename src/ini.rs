use crate::http::{RequestArgs, Url};
use crate::stdio::{ask_binary, ask_path, ask_string};
use crate::utils::Result;

use ini::{Ini, Properties};
use std::collections::HashMap;

pub const DEFAULT_INI_FILE_PATH: &str = "~/.wiq";

const INI_HOST: &str = "host";
const INI_USER: &str = "user";
const INI_PASSWORD: &str = "password";
const INI_CA_CERT: &str = "ca_cert";
const INI_INSECURE: &str = "insecure";

#[derive(Debug)]
pub struct IniFileArgs {
    url: Option<Url>,
    user: Option<String>,
    password: Option<String>,
    insecure: bool,
    ca_cert: Option<String>,
    headers: HashMap<String, String>,
}

impl RequestArgs for IniFileArgs {
    fn user(&self) -> Option<&String> {
        self.user.as_ref()
    }

    fn password(&self) -> Option<&String> {
        self.password.as_ref()
    }

    fn insecure(&self) -> bool {
        self.insecure
    }

    fn ca_cert(&self) -> Option<&String> {
        self.ca_cert.as_ref()
    }

    fn method(&self) -> Option<&String> {
        None
    }

    fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    fn body(&self) -> Option<&String> {
        None
    }

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

impl IniFileArgs {
    #[allow(dead_code)]
    pub fn exists(file_path: &str, name: &str) -> bool {
        let extended_path = shellexpand::tilde(file_path).to_string();
        if !std::path::Path::new(&extended_path).exists() {
            return false;
        }
        let ini = Ini::load_from_file(extended_path).unwrap();
        ini.section(Some(name)).is_some()
    }

    pub fn load(file_path: &str, name: &str) -> Result<Option<IniFileArgs>> {
        let extended_path = shellexpand::tilde(file_path).to_string();
        if !std::path::Path::new(&extended_path).exists() {
            dbg!("file not found: {}", &extended_path);
            return Ok(None);
        }
        let ini = Ini::load_from_file(extended_path)?;
        let section = match ini.section(Some(name)) {
            Some(s) => s,
            None => return Ok(None),
        };

        let mut headers = HashMap::<String, String>::new();
        for (key, value) in section.iter() {
            // pick up header entries only
            if let Some(stripped) = key.strip_prefix("@") {
                headers.insert(stripped.to_string(), value.to_string());
            }
        }

        fn try_get<T>(section: &Properties, key: &str) -> Option<T>
        where
            T: std::str::FromStr,
            T::Err: std::fmt::Debug,
        {
            section.get(key).map(|s| s.parse::<T>().unwrap())
        }

        let url = match try_get::<String>(section, INI_HOST) {
            Some(s) => Some(Url::parse(&s)),
            None => None,
        };

        let profile = IniFileArgs {
            url: url,
            user: try_get(section, INI_USER),
            password: try_get(section, INI_PASSWORD),
            insecure: try_get::<bool>(section, INI_INSECURE).unwrap_or(false),
            ca_cert: try_get(section, INI_CA_CERT),
            headers: headers.clone(),
        };

        Ok(Some(profile))
    }

    #[allow(dead_code)]
    pub fn ask() -> Result<IniFileArgs> {
        let i = std::io::stdin();
        let url = ask_string(&i, "host: ")?;
        let user = if ask_binary(&i, "Do you need a user/password for this URL? [y/N]: ")? {
            Some(ask_string(&i, "user: ")?)
        } else {
            None
        };
        let password = if user.is_some() {
            Some(ask_string(&i, "password: ")?)
        } else {
            None
        };

        let ca_cert = if url.starts_with("https")
            && ask_binary(&i, "Do you need to use a custom CA certificate? [y/N]: ")?
        {
            let path = ask_path(&i, "CA certificate file: ")?;
            Some(path)
        } else {
            None
        };

        Ok(IniFileArgs {
            url: Some(Url::parse(&url)),
            user,
            password,
            insecure: false,
            ca_cert,
            headers: HashMap::new(),
        })
    }

    #[allow(dead_code)]
    pub fn put(file_path: &str, name: &str, profile: &IniFileArgs) -> Result<()> {
        let mut conf = Ini::new();
        let sect_name = Some(name.to_string());
        let mut sect = conf.with_section(sect_name);

        if profile.url().is_some() {
            sect.set(INI_HOST, profile.url().unwrap().to_string());
        }
        if profile.user().is_some() {
            sect.set(INI_USER, profile.user().unwrap());
        }
        if profile.password().is_some() {
            sect.set(INI_PASSWORD, profile.password().unwrap());
        }
        sect.set(INI_INSECURE, profile.insecure().to_string());

        if profile.ca_cert().is_some() {
            sect.set(INI_CA_CERT, profile.ca_cert().unwrap());
        }

        for (k, v) in profile.headers.iter() {
            sect.set(format!("@{}", k), v);
        }

        let p = shellexpand::tilde(file_path).to_string();
        conf.write_to_file(p).unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::remove_file;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempPath};

    const TEST_SERVER: &str = "http://test-server";
    const TEST_USER: &str = "test_user";
    const TEST_PASSWORD: &str = "test_password";
    const TEST_CA_CERT: &str = "/etc/pki/ca/cert.crt";
    const TEST_API_KEY: &str = "ABCDE";
    const TEST_CONTENT_TYPE: &str = "application/json";
    const TEST_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
    const DEFAULT_INI_SECTION: &str = "default";

    fn create_ini_file() -> Result<TempPath> {
        let content = format!(
            "[{}]\n\
             host={}\n\
             user={}\n\
             password={}\n\
             ca_cert={}\n\
             insecure=false\n\
             api_key={}\n\
             @Content-Type={}\n\
             @User-Agent={}\n\
             ",
            DEFAULT_INI_SECTION,
            TEST_SERVER,
            TEST_USER,
            TEST_PASSWORD,
            TEST_CA_CERT,
            TEST_API_KEY,
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

    fn test_profile(path: &str) -> Result<IniFileArgs> {
        let profile = IniFileArgs::load(&path, DEFAULT_INI_SECTION)?.unwrap();

        assert_eq!(
            profile.url().map(|u| u.to_string()),
            Some(TEST_SERVER.to_string())
        );
        assert_eq!(profile.user(), Some(&TEST_USER.to_string()));
        assert_eq!(profile.password(), Some(&TEST_PASSWORD.to_string()));
        assert_eq!(profile.ca_cert(), Some(&TEST_CA_CERT.to_string()));
        assert_eq!(profile.insecure(), false);

        assert_eq!(profile.headers.len(), 2);
        assert_eq!(
            profile.headers["Content-Type"],
            TEST_CONTENT_TYPE.to_string()
        );
        assert_eq!(profile.headers["User-Agent"], TEST_USER_AGENT.to_string());

        Ok(profile)
    }

    #[test]
    fn test_load_profile() -> Result<()> {
        let temp_path = create_ini_file()?;
        let path = temp_path.as_os_str().to_str().unwrap().to_string();
        let _ = test_profile(&path)?;
        temp_path.close()?;
        Ok(())
    }

    #[test]
    fn test_add_profile() -> Result<()> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), TEST_CONTENT_TYPE.to_string());
        headers.insert("User-Agent".to_string(), TEST_USER_AGENT.to_string());

        let profile = IniFileArgs {
            url: Some(Url::parse(TEST_SERVER)),
            user: Some(TEST_USER.to_string()),
            password: Some(TEST_PASSWORD.to_string()),
            insecure: false,
            ca_cert: Some(TEST_CA_CERT.to_string()),
            headers: headers,
        };
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap().to_string();

        let _ = IniFileArgs::put(&path, DEFAULT_INI_SECTION, &profile)?;
        let _ = test_profile(&path)?;
        let _ = delete_ini_file(&path)?;

        Ok(())
    }
}
