use crate::cookie::{Cookie, CookieCreate, CookieManager, CookieUpdate, Result};
use std::sync::OnceLock;

static COOKIE_MANAGER: OnceLock<CookieManager> = OnceLock::new();

#[flutter_rust_bridge::frb(sync)]
pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

#[flutter_rust_bridge::frb(sync)]
pub fn init_cookie_manager_in_memory() -> Result<()> {
    let manager = CookieManager::new_in_memory()?;
    COOKIE_MANAGER.set(manager).map_err(|_| {
        crate::cookie::CookieError::InvalidCookie("Cookie manager already initialized".to_string())
    })?;
    Ok(())
}

#[flutter_rust_bridge::frb(sync)]
pub fn init_cookie_manager_from_path(path: String) -> Result<()> {
    let manager = CookieManager::new_from_path(&path)?;
    COOKIE_MANAGER.set(manager).map_err(|_| {
        crate::cookie::CookieError::InvalidCookie("Cookie manager already initialized".to_string())
    })?;
    Ok(())
}

#[flutter_rust_bridge::frb(sync)]
pub fn create_cookie(cookie: CookieCreate) -> Result<i64> {
    let manager = COOKIE_MANAGER
        .get()
        .ok_or_else(|| crate::cookie::CookieError::InvalidCookie("Cookie manager not initialized".to_string()))?;
    manager.create_cookie(cookie)
}

#[flutter_rust_bridge::frb(sync)]
pub fn get_cookie(id: i64) -> Result<Option<Cookie>> {
    let manager = COOKIE_MANAGER
        .get()
        .ok_or_else(|| crate::cookie::CookieError::InvalidCookie("Cookie manager not initialized".to_string()))?;
    manager.get_cookie(id)
}

#[flutter_rust_bridge::frb(sync)]
pub fn get_cookies_by_domain(domain: String) -> Result<Vec<Cookie>> {
    let manager = COOKIE_MANAGER
        .get()
        .ok_or_else(|| crate::cookie::CookieError::InvalidCookie("Cookie manager not initialized".to_string()))?;
    manager.get_cookies_by_domain(&domain)
}

#[flutter_rust_bridge::frb(sync)]
pub fn get_all_cookies() -> Result<Vec<Cookie>> {
    let manager = COOKIE_MANAGER
        .get()
        .ok_or_else(|| crate::cookie::CookieError::InvalidCookie("Cookie manager not initialized".to_string()))?;
    manager.get_all_cookies()
}

#[flutter_rust_bridge::frb(sync)]
pub fn update_cookie(id: i64, update: CookieUpdate) -> Result<()> {
    let manager = COOKIE_MANAGER
        .get()
        .ok_or_else(|| crate::cookie::CookieError::InvalidCookie("Cookie manager not initialized".to_string()))?;
    manager.update_cookie(id, update)
}

#[flutter_rust_bridge::frb(sync)]
pub fn delete_cookie(id: i64) -> Result<()> {
    let manager = COOKIE_MANAGER
        .get()
        .ok_or_else(|| crate::cookie::CookieError::InvalidCookie("Cookie manager not initialized".to_string()))?;
    manager.delete_cookie(id)
}

#[flutter_rust_bridge::frb(sync)]
pub fn delete_cookies_by_domain(domain: String) -> Result<()> {
    let manager = COOKIE_MANAGER
        .get()
        .ok_or_else(|| crate::cookie::CookieError::InvalidCookie("Cookie manager not initialized".to_string()))?;
    manager.delete_cookies_by_domain(&domain)
}

#[flutter_rust_bridge::frb(sync)]
pub fn delete_all_cookies() -> Result<()> {
    let manager = COOKIE_MANAGER
        .get()
        .ok_or_else(|| crate::cookie::CookieError::InvalidCookie("Cookie manager not initialized".to_string()))?;
    manager.delete_all_cookies()
}

#[flutter_rust_bridge::frb(sync)]
pub fn get_cookies_for_url(url: String) -> Result<Vec<Cookie>> {
    let manager = COOKIE_MANAGER
        .get()
        .ok_or_else(|| crate::cookie::CookieError::InvalidCookie("Cookie manager not initialized".to_string()))?;
    manager.get_cookies_for_url(&url)
}
