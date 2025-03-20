use ini::Ini;
use std::path::Path;

const DEFAULT_SECTION: &str = "default";

const INI_HOST: &str = "host";
const INI_USER: &str = "user";
const INI_PASSWORD: &str = "password";
const INI_API_KEY: &str = "api_key";
const INI_CA_CERT: &str = "ca_cert";
const INI_INSECURE: &str = "insecure";
const INI_CONTENT_TYPE: &str = "content_type";

#[derive(Debug)]
pub struct IniSectionArgs {
    host: Option<String>,
    user: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
    content_type: Option<String>,
    insecure: bool,
    ca_cert: Option<String>,
}

impl IniSectionArgs {
    pub fn exists(file: &str, section: &str) -> bool {
        let ini_file = shellexpand::tilde(file).to_string();
        if !Path::new(&ini_file).exists() {
            return false
        }

        let ini = Ini::load_from_file(ini_file).unwrap();
        match ini.section(Some(section)) {
            Some(_) => true,
            None => false
        }
    }

    pub fn from_file(file: &str, section: &str) -> Option<Self> {
        let ini_file = shellexpand::tilde(file).to_string();
        let ini = match Ini::load_from_file(ini_file) {
            Ok(i) => i,
            Err(_) => return None,
        };

        let section = match ini.section(Some(section)) {
            Some(s) => s,
            None => return None,
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

        Some(IniSectionArgs {
            host,
            user,
            password,
            api_key,
            content_type,
            insecure,
            ca_cert,
        })
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

