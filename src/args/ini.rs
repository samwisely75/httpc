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

