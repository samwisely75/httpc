use ini::Ini;
use std::path::Path;

pub const DEFAULT_INI_FILE_PATH: &str = "~/.http";
pub const DEFAULT_INI_SECTION: &str = "default";

const INI_HOST: &str = "host";
const INI_USER: &str = "user";
const INI_PASSWORD: &str = "password";
const INI_API_KEY: &str = "api_key";
const INI_CA_CERT: &str = "ca_cert";
const INI_INSECURE: &str = "insecure";
const INI_CONTENT_TYPE: &str = "content_type";

#[derive(Debug)]
pub struct Profile {
    host: Option<String>,
    user: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
    content_type: Option<String>,
    insecure: bool,
    ca_cert: Option<String>,
}

impl Profile {
    pub fn new(
        host: Option<String>,
        user: Option<String>,
        password: Option<String>,
        api_key: Option<String>,
        content_type: Option<String>,
        insecure: bool,
        ca_cert: Option<String>,
    ) -> Self {
        Profile {
            host,
            user,
            password,
            api_key,
            content_type,
            insecure,
            ca_cert,
        }
    }

    pub fn host(&self) -> Option<String> {
        self.host.clone().map(|s| s.to_string())
    }

    pub fn user(&self) -> Option<String> {
        self.user.clone().map(|s| s.to_string())
    }

    pub fn password(&self) -> Option<String> {
        self.password.clone().map(|s| s.to_string())
    }

    pub fn api_key(&self) -> Option<String> {
        self.api_key.clone().map(|s| s.to_string())
    }

    pub fn insecure(&self) -> bool {
        self.insecure
    }

    pub fn ca_cert(&self) -> Option<String> {
        self.ca_cert.clone().map(|s| s.to_string())
    }

    pub fn content_type(&self) -> Option<String> {
        self.content_type.clone().map(|s| s.to_string())
    }
}

pub struct IniFile;

impl IniFile {
    pub fn profile_exists(file_path: &str, name: &str) -> bool {
        let ini_file = shellexpand::tilde(file_path).to_string();
        if !Path::new(&ini_file).exists() {
            return false;
        }
        Ini::load_from_file(file_path)
            .unwrap()
            .section(Some(name))
            .is_some()
    }

    pub fn load_profile(
        file_path: &str,
        name: &str,
    ) -> Result<Option<Profile>, Box<dyn std::error::Error>> {
        let file_path = shellexpand::tilde(file_path).to_string();

        if !Path::new(&file_path).exists() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Load error: file not found ({})", file_path),
            )));
        }

        let ini = Ini::load_from_file(file_path)?;
        let section = match ini.section(Some(name)) {
            Some(s) => s,
            None => return Ok(None),
        };

        let host = section.get(INI_HOST).map(|s| s.to_string());
        let user = section.get(INI_USER).map(|s| s.to_string());
        let password = section.get(INI_PASSWORD).map(|s| s.to_string());
        let api_key = section.get(INI_API_KEY).map(|s| s.to_string());
        let content_type = section.get(INI_CONTENT_TYPE).map(|s| s.to_string());
        let insecure = section
            .get(INI_INSECURE)
            .map(|s| s.parse::<bool>().unwrap())
            .unwrap_or(false);
        let ca_cert = section.get(INI_CA_CERT).map(|s| s.to_string());

        let profile = Profile {
            host: host,
            user: user,
            password: password,
            api_key: api_key,
            content_type: content_type,
            insecure: insecure,
            ca_cert: ca_cert,
        };

        Ok(Some(profile))
    }

    pub fn add_profile(
        file_path: &str,
        name: &str,
        profile: &Profile,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
        if profile.content_type().is_some() {
            sect.set(INI_CONTENT_TYPE, profile.content_type().unwrap());
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

    const INI_FILE: &str = "~/.http_test";
    const TEST_SERVER: &str = "http://test-server";
    const TEST_USER: &str = "test_user";
    const TEST_PASSWORD: &str = "test_password";
    const TEST_CA_CERT: &str = "/etc/pki/ca/cert.crt";
    const TEST_API_KEY: &str = "ABCDE";
    const TEST_CONTENT_TYPE: &str = "application/json";

    fn create_ini_file() -> Result<TempPath, Box<dyn std::error::Error>> {
        let content = format!(
            "[{}]\n\
             host={}\n\
             user={}\n\
             password={}\n\
             ca_cert={}\n\
             insecure=false\n\
             api_key={}\n\
             content_type={}\n\
             ",
            DEFAULT_INI_SECTION,
            TEST_SERVER,
            TEST_USER,
            TEST_PASSWORD,
            TEST_CA_CERT,
            TEST_API_KEY,
            TEST_CONTENT_TYPE
        );

        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;
        let path = file.into_temp_path();
        Ok(path)
    }

    fn delete_ini_file(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = shellexpand::tilde(file_path).to_string();
        if Path::new(&path).exists() {
            remove_file(path)?;
        }
        Ok(())
    }

    fn test_profile(path: &str) -> Result<Profile, Box<dyn std::error::Error>> {
        let profile = IniFile::load_profile(&path, DEFAULT_INI_SECTION)?.unwrap();

        assert_eq!(profile.host(), Some(TEST_SERVER.to_string()));
        assert_eq!(profile.user(), Some(TEST_USER.to_string()));
        assert_eq!(profile.password(), Some(TEST_PASSWORD.to_string()));
        assert_eq!(profile.ca_cert(), Some(TEST_CA_CERT.to_string()));
        assert_eq!(profile.api_key(), Some(TEST_API_KEY.to_string()));
        assert_eq!(profile.content_type(), Some(TEST_CONTENT_TYPE.to_string()));
        assert_eq!(profile.insecure(), false);

        Ok(profile)
    }

    #[test]
    fn test_load_profile() -> Result<(), Box<dyn std::error::Error>> {
        let temp_path = create_ini_file()?;
        let path = temp_path.as_os_str().to_str().unwrap().to_string();
        let _ = test_profile(&path)?;
        temp_path.close()?;
        Ok(())
    }

    #[test]
    fn test_add_profile() -> Result<(), Box<dyn std::error::Error>> {
        let profile = Profile {
            host: Some(TEST_SERVER.to_string()),
            user: Some(TEST_USER.to_string()),
            password: Some(TEST_PASSWORD.to_string()),
            api_key: Some(TEST_API_KEY.to_string()),
            content_type: Some(TEST_CONTENT_TYPE.to_string()),
            insecure: false,
            ca_cert: Some(TEST_CA_CERT.to_string()),
        };
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap().to_string();

        let _ = IniFile::add_profile(&path, DEFAULT_INI_SECTION, &profile)?;
        let _ = test_profile(&path)?;
        let _ = delete_ini_file(INI_FILE)?;

        Ok(())
    }
}
