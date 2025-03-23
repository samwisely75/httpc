use crate::utils::{Result, ask_binary, ask_path, ask_string};
use ini::{Ini, Properties};
use std::{collections::HashMap, path::Path};

pub const DEFAULT_INI_FILE_PATH: &str = "~/.wiq";
pub const DEFAULT_INI_SECTION: &str = "default";

const INI_HOST: &str = "host";
const INI_USER: &str = "user";
const INI_PASSWORD: &str = "password";
const INI_API_KEY: &str = "api_key";
const INI_CA_CERT: &str = "ca_cert";
const INI_INSECURE: &str = "insecure";

#[derive(Debug)]
pub struct Profile {
    host: Option<String>,
    user: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
    insecure: bool,
    ca_cert: Option<String>,
    pub headers: HashMap<String, String>,
}

impl Profile {
    pub fn new(
        host: Option<String>,
        user: Option<String>,
        password: Option<String>,
        api_key: Option<String>,
        insecure: bool,
        ca_cert: Option<String>,
        headers: HashMap<String, String>,
    ) -> Self {
        Profile {
            host,
            user,
            password,
            api_key,
            insecure,
            ca_cert,
            headers: headers,
        }
    }

    pub fn host(&self) -> Option<String> {
        if self.host.is_some() && self.host.clone().unwrap().ends_with("/") {
            let mut h = self.host.clone().unwrap();
            h.pop();
            Some(h)
        } else {
            self.host.clone()
        }
    }

    pub fn user(&self) -> Option<String> {
        self.user.clone()
    }

    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }

    pub fn api_key(&self) -> Option<String> {
        self.api_key.clone()
    }

    pub fn insecure(&self) -> bool {
        self.insecure
    }

    pub fn ca_cert(&self) -> Option<String> {
        self.ca_cert.clone()
    }
}

pub struct IniFile;

impl IniFile {
    pub fn profile_exists(file_path: &str, name: &str) -> bool {
        let extended_path = shellexpand::tilde(file_path).to_string();
        if !Path::new(&extended_path).exists() {
            return false;
        }
        let ini = Ini::load_from_file(extended_path).unwrap();
        ini.section(Some(name)).is_some()
    }

    pub fn load_profile(file_path: &str, name: &str) -> Result<Option<Profile>> {
        let extended_path = shellexpand::tilde(file_path).to_string();
        if !Path::new(&extended_path).exists() {
            dbg!("file not found: {}", &extended_path);
            return Ok(None);
        }
        let ini = Ini::load_from_file(extended_path)?;
        let section = match ini.section(Some(name)) {
            Some(s) => s,
            None => return Ok(None),
        };

        fn try_get<T>(section: &Properties, key: &str) -> Option<T>
        where
            T: std::str::FromStr,
            T::Err: std::fmt::Debug,
        {
            section.get(key).map(|s| s.parse::<T>().unwrap())
        }

        let headers = section
            .iter()
            .filter(|(key, _)| key.starts_with("header:"))
            .map(|(key, value)| (key[7..].to_string(), value.to_string()))
            .collect::<HashMap<String, String>>();

        let profile = Profile {
            host: try_get(&section, INI_HOST),
            user: try_get(&section, INI_USER),
            password: try_get(&section, INI_PASSWORD),
            api_key: try_get(&section, INI_API_KEY),
            insecure: try_get::<bool>(&section, INI_INSECURE).unwrap_or(false),
            ca_cert: try_get(&section, INI_CA_CERT),
            headers: headers,
        };

        Ok(Some(profile))
    }

    pub fn ask_profile() -> Result<Profile> {
        let i = std::io::stdin();
        let host = ask_string(&i, "host: ")?;
        let user = if ask_binary(&i, "Do you want to use a user? [y/N]: ")? {
            Some(ask_string(&i, "user: ")?)
        } else {
            None
        };
        let password = if user.is_some() {
            Some(ask_string(&i, "password: ")?)
        } else {
            None
        };
        let api_key = if user.is_none() && ask_binary(&i, "Do you want to use an API key? [y/N]: ")?
        {
            Some(ask_string(&i, "API key: ")?)
        } else {
            None
        };

        let ca_cert = if host.starts_with("https")
            && ask_binary(&i, "Do you need to use a custom CA certificate? [y/N]: ")?
        {
            let path = ask_path(&i, "CA certificate file: ")?;
            Some(path)
        } else {
            None
        };

        Ok(Profile::new(
            Some(host),
            user,
            password,
            api_key,
            false,
            ca_cert,
            HashMap::new(),
        ))
    }

    pub fn add_profile(file_path: &str, name: &str, profile: &Profile) -> Result<()> {
        let mut conf = Ini::new();
        let sect_name = Some(name.to_string());
        let mut sect = conf.with_section(sect_name);

        if profile.host().is_some() {
            sect.set(INI_HOST, profile.host().unwrap());
        }
        if profile.user().is_some() {
            sect.set(INI_USER, profile.user().unwrap());
        }
        if profile.password().is_some() {
            sect.set(INI_PASSWORD, profile.password().unwrap());
        }
        if profile.api_key().is_some() {
            sect.set(INI_API_KEY, profile.api_key().unwrap());
        }
        sect.set(INI_INSECURE, profile.insecure().to_string());

        if profile.ca_cert().is_some() {
            sect.set(INI_CA_CERT, profile.ca_cert().unwrap());
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

    fn create_ini_file() -> Result<TempPath> {
        let content = format!(
            "[{}]\n\
             host={}\n\
             user={}\n\
             password={}\n\
             ca_cert={}\n\
             insecure=false\n\
             api_key={}\n\
             header::Content-Type={}\n\
             header::User-Agent={}\n\
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
        if Path::new(&path).exists() {
            remove_file(path)?;
        }
        Ok(())
    }

    fn test_profile(path: &str) -> Result<Profile> {
        let profile = IniFile::load_profile(&path, DEFAULT_INI_SECTION)?.unwrap();

        assert_eq!(profile.host(), Some(TEST_SERVER.to_string()));
        assert_eq!(profile.user(), Some(TEST_USER.to_string()));
        assert_eq!(profile.password(), Some(TEST_PASSWORD.to_string()));
        assert_eq!(profile.ca_cert(), Some(TEST_CA_CERT.to_string()));
        assert_eq!(profile.api_key(), Some(TEST_API_KEY.to_string()));
        assert_eq!(profile.insecure(), false);
        assert_eq!(
            profile.headers.get("Content-Type"),
            Some(&TEST_CONTENT_TYPE.to_string())
        );
        assert_eq!(
            profile.headers.get("User-Agent"),
            Some(&TEST_USER_AGENT.to_string())
        );

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
        let profile = Profile {
            host: Some(TEST_SERVER.to_string()),
            user: Some(TEST_USER.to_string()),
            password: Some(TEST_PASSWORD.to_string()),
            api_key: Some(TEST_API_KEY.to_string()),
            insecure: false,
            ca_cert: Some(TEST_CA_CERT.to_string()),
            headers: HashMap::new(),
        };
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap().to_string();

        let _ = IniFile::add_profile(&path, DEFAULT_INI_SECTION, &profile)?;
        let _ = test_profile(&path)?;
        let _ = delete_ini_file(&path)?;

        Ok(())
    }
}
