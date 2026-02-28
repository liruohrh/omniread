use crate::cookie::error::{CookieError, Result};
use crate::cookie::model::{Cookie, CookieCreate, CookieUpdate};
use crate::cookie::storage::CookieStorage;
use std::sync::{Arc, Mutex};

pub struct CookieManager {
    storage: Arc<Mutex<CookieStorage>>,
}

impl CookieManager {
    pub fn new(storage: CookieStorage) -> Self {
        Self {
            storage: Arc::new(Mutex::new(storage)),
        }
    }

    pub fn new_in_memory() -> Result<Self> {
        Ok(Self::new(CookieStorage::new_in_memory()?))
    }

    pub fn new_from_path(path: &str) -> Result<Self> {
        Ok(Self::new(CookieStorage::new_from_path(path)?))
    }

    pub fn create_cookie(&self, cookie: CookieCreate) -> Result<i64> {
        self.storage.lock().unwrap().insert(cookie)
    }

    pub fn get_cookie(&self, id: i64) -> Result<Option<Cookie>> {
        self.storage.lock().unwrap().get(id)
    }

    pub fn get_cookies_by_domain(&self, domain: &str) -> Result<Vec<Cookie>> {
        self.storage.lock().unwrap().get_by_domain(domain)
    }

    pub fn get_all_cookies(&self) -> Result<Vec<Cookie>> {
        self.storage.lock().unwrap().get_all()
    }

    pub fn update_cookie(&self, id: i64, update: CookieUpdate) -> Result<()> {
        let storage = self.storage.lock().unwrap();
        if storage.get(id)?.is_none() {
            return Err(CookieError::NotFound(format!("Cookie with id {}", id)));
        }
        storage.update(id, update)
    }

    pub fn delete_cookie(&self, id: i64) -> Result<()> {
        let storage = self.storage.lock().unwrap();
        if storage.get(id)?.is_none() {
            return Err(CookieError::NotFound(format!("Cookie with id {}", id)));
        }
        storage.delete(id)
    }

    pub fn delete_cookies_by_domain(&self, domain: &str) -> Result<()> {
        self.storage.lock().unwrap().delete_by_domain(domain)
    }

    pub fn delete_all_cookies(&self) -> Result<()> {
        self.storage.lock().unwrap().delete_all()
    }

    pub fn get_cookies_for_url(&self, url: &str) -> Result<Vec<Cookie>> {
        let parsed_url = url::Url::parse(url)?;
        let domain = parsed_url
            .domain()
            .ok_or_else(|| CookieError::InvalidCookie("Invalid URL domain".to_string()))?;
        self.get_cookies_by_domain(domain)
    }
}
