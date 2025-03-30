#[derive(Debug)]
pub struct HttpRequest {
    method: Option<String>,
    url: Option<Url>,
    body: Option<String>,
    user: Option<String>,
    password: Option<String>,
    insecure: bool,
    ca_cert: Option<String>,
    headers: HashMap<String, String>,
}

impl Clone for HttpRequest {
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            url: self.url.clone(),
            body: self.body.clone(),
            user: self.user.clone(),
            password: self.password.clone(),
            insecure: self.insecure.clone(),
            ca_cert: self.ca_cert.clone(),
            headers: self.headers.clone(),
        }
    }
}

impl HttpRequestArgs for HttpRequest {
    fn method(&self) -> Option<&String> {
        self.method.as_ref()
    }

    fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    fn body(&self) -> Option<&String> {
        self.body.as_ref()
    }

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

    fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

impl HttpRequest {
    pub fn from<T>(args: &T) -> Self
    where
        T: HttpRequestArgs,
    {
        Self {
            method: args.method().map(|s| s.to_string()),
            url: args.url().map(|u| Url::parse(u.to_string().as_str())),
            body: args.body().map(|s| s.to_string()),
            user: args.user().map(|s| s.to_string()),
            password: args.password().map(|s| s.to_string()),
            insecure: args.insecure(),
            ca_cert: args.ca_cert().map(|s| s.to_string()),
            headers: args.headers().clone(),
        }
    }

    pub fn merge<T>(&mut self, args: &T) -> &mut Self
    where
        T: HttpRequestArgs,
    {
        if let Some(url) = args.url() {
            if self.url.is_none() {
                self.url = Some(Url::parse(url.to_string().as_str()));
            } else {
                self.url.as_mut().unwrap().merge(url);
            }
        }

        if let Some(method) = args.method() {
            self.method = Some(method.to_string());
        }

        if let Some(body) = args.body() {
            self.body = Some(body.to_string());
        }

        if let Some(user) = args.user() {
            self.user = Some(user.to_string());
        }

        if let Some(password) = args.password() {
            self.password = Some(password.to_string());
        }

        if let Some(ca_cert) = args.ca_cert() {
            self.ca_cert = Some(ca_cert.to_string());
        }

        self.insecure = args.insecure();

        args.headers().iter().for_each(|(key, value)| {
            self.headers.insert(key.to_string(), value.to_string());
        });

        self
    }
}

