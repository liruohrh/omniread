use crate::cookie::CookieManager;
use crate::http_client::error::{HttpClientError, Result};
use crate::request_config::{RequestConfig, RequestMethod};
use reqwest::{header, Client, ClientBuilder};
use std::collections::HashMap;
use std::sync::Arc;

/// Full HTTP response including status, body, and headers
#[derive(Debug, Clone)]
pub struct FullHttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

pub struct HttpClient {
    client: Client,
    cookie_manager: Option<Arc<CookieManager>>,
    default_headers: HashMap<String, String>,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        let client = ClientBuilder::new()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let mut default_headers = HashMap::new();
        default_headers.insert(
            header::ACCEPT.to_string(),
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".to_string(),
        );
        default_headers.insert(
            header::ACCEPT_LANGUAGE.to_string(),
            "zh-CN,zh;q=0.9,en;q=0.8,zh-TW;q=0.7,zh-HK;q=0.6".to_string(),
        );
        default_headers.insert(header::CACHE_CONTROL.to_string(), "max-age=0".to_string());

        Ok(Self {
            client,
            cookie_manager: None,
            default_headers,
        })
    }

    pub fn with_cookie_manager(cookie_manager: Arc<CookieManager>) -> Result<Self> {
        let mut client = Self::new()?;
        client.cookie_manager = Some(cookie_manager);
        Ok(client)
    }

    pub fn set_default_header(&mut self, key: String, value: String) {
        self.default_headers.insert(key, value);
    }

    pub async fn get(&self, url: &str) -> Result<String> {
        self.execute(RequestConfig::new(url.to_string())).await
    }

    pub async fn get_with_headers(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<String> {
        let mut config = RequestConfig::new(url.to_string());
        for (key, value) in headers {
            config.headers.insert(key, value);
        }
        self.execute(config).await
    }

    pub async fn execute(&self, config: RequestConfig) -> Result<String> {
        let parsed_url = url::Url::parse(&config.url)?;

        let mut request = match config.method {
            RequestMethod::Get => self.client.get(&config.url),
            RequestMethod::Post => {
                let mut req = self.client.post(&config.url);
                if let Some(content_type) = &config.content_type {
                    req = req.header(header::CONTENT_TYPE, content_type);
                }
                if let Some(body) = &config.body {
                    req = req.json(body);
                }
                req
            }
            RequestMethod::Put => {
                let mut req = self.client.put(&config.url);
                if let Some(content_type) = &config.content_type {
                    req = req.header(header::CONTENT_TYPE, content_type);
                }
                if let Some(body) = &config.body {
                    req = req.json(body);
                }
                req
            }
            RequestMethod::Delete => self.client.delete(&config.url),
            RequestMethod::Patch => {
                let mut req = self.client.patch(&config.url);
                if let Some(content_type) = &config.content_type {
                    req = req.header(header::CONTENT_TYPE, content_type);
                }
                if let Some(body) = &config.body {
                    req = req.json(body);
                }
                req
            }
        };

        for (key, value) in &self.default_headers {
            if !config.headers.contains_key(key) {
                request = request.header(key, value);
            }
        }

        for (key, value) in &config.headers {
            request = request.header(key, value);
        }

        if let Some(cm) = &self.cookie_manager {
            if let Some(domain) = parsed_url.domain() {
                let cookies = cm.get_cookies_by_domain(domain)?;
                for cookie in cookies {
                    request =
                        request.header(header::COOKIE, format!("{}={}", cookie.name, cookie.value));
                }
            }
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(HttpClientError::RequestFailed(response.status().as_u16()));
        }

        let body = response.text().await?;
        Ok(body)
    }

    pub async fn execute_full(&self, config: RequestConfig) -> Result<FullHttpResponse> {
        let parsed_url = url::Url::parse(&config.url)?;

        let mut request = match config.method {
            RequestMethod::Get => self.client.get(&config.url),
            RequestMethod::Post => {
                let mut req = self.client.post(&config.url);
                if let Some(content_type) = &config.content_type {
                    req = req.header(header::CONTENT_TYPE, content_type);
                }
                if let Some(body) = &config.body {
                    req = req.json(body);
                }
                req
            }
            RequestMethod::Put => {
                let mut req = self.client.put(&config.url);
                if let Some(content_type) = &config.content_type {
                    req = req.header(header::CONTENT_TYPE, content_type);
                }
                if let Some(body) = &config.body {
                    req = req.json(body);
                }
                req
            }
            RequestMethod::Delete => self.client.delete(&config.url),
            RequestMethod::Patch => {
                let mut req = self.client.patch(&config.url);
                if let Some(content_type) = &config.content_type {
                    req = req.header(header::CONTENT_TYPE, content_type);
                }
                if let Some(body) = &config.body {
                    req = req.json(body);
                }
                req
            }
        };

        for (key, value) in &self.default_headers {
            if !config.headers.contains_key(key) {
                request = request.header(key, value);
            }
        }

        for (key, value) in &config.headers {
            request = request.header(key, value);
        }

        if let Some(cm) = &self.cookie_manager {
            if let Some(domain) = parsed_url.domain() {
                let cookies = cm.get_cookies_by_domain(domain)?;
                for cookie in cookies {
                    request = request.header(header::COOKIE, format!("{}={}", cookie.name, cookie.value));
                }
            }
        }

        let response = request.send().await?;

        let status = response.status().as_u16();

        let mut headers = HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(val_str) = value.to_str() {
                headers.insert(name.as_str().to_string(), val_str.to_string());
            }
        }

        let body = response.text().await?;

        Ok(FullHttpResponse {
            status,
            body,
            headers,
        })
    }
}
