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
    pub fn from_file(file: &str, section: &str) -> Option<Self> {
        let ini_file = shellexpand::tilde(file).to_string();
        let ini = match ini::Ini::load_from_file(ini_file) {
            Ok(i) => i,
            Err(_) => return None,
        };

        let section = match ini.section(Some(section)) {
            Some(s) => s,
            None => return None,
        };

        let host = section.get("host").map(|s| s.to_string());
        let user = section.get("user").map(|s| s.to_string());
        let password = section.get("password").map(|s| s.to_string());
        let api_key = section.get("api_key").map(|s| s.to_string());
        let content_type = section.get("content_type").map(|s| s.to_string());
        let insecure = section
            .get("insecure")
            .map(|s| s.parse::<bool>().unwrap())
            .unwrap_or(false);
        let ca_cert = section.get("ca_cert").map(|s| s.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ini_section_args() {
        let ini = IniSectionArgs::from_file("/Users/satoshi/.www", "test");
        assert_eq!(ini.is_some(), true);
        let ini = ini.unwrap();
        assert_eq!(ini.host(), Some("http://localhost".to_string()));
        assert_eq!(ini.user(), Some("user".to_string()));
        assert_eq!(ini.password(), Some("password".to_string()));
        assert_eq!(ini.api_key(), Some("api_key".to_string()));
        assert_eq!(ini.content_type(), Some("application/json".to_string()));
        assert_eq!(ini.insecure(), true);
        assert_eq!(ini.ca_cert(), Some("ca_cert.pem".to_string()));
    }
}
