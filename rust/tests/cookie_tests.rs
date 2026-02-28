#[cfg(test)]
mod tests {
    use rust_lib_omniread::cookie::{CookieCreate, CookieManager, CookieUpdate};

    #[test]
    fn test_cookie_crud() {
        let manager = CookieManager::new_in_memory().unwrap();

        let cookie_create = CookieCreate {
            name: "session_id".to_string(),
            value: "abc123".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: true,
            same_site: Some("Lax".to_string()),
            expires: None,
        };

        let cookie_id = manager.create_cookie(cookie_create).unwrap();
        assert!(cookie_id > 0);

        let cookie = manager.get_cookie(cookie_id).unwrap().unwrap();
        assert_eq!(cookie.name, "session_id");
        assert_eq!(cookie.value, "abc123");
        assert_eq!(cookie.domain, "example.com");

        let update = CookieUpdate {
            name: None,
            value: Some("def456".to_string()),
            path: None,
            secure: None,
            http_only: None,
            same_site: None,
            expires: None,
        };
        manager.update_cookie(cookie_id, update).unwrap();

        let updated_cookie = manager.get_cookie(cookie_id).unwrap().unwrap();
        assert_eq!(updated_cookie.value, "def456");

        let cookies = manager.get_cookies_by_domain("example.com").unwrap();
        assert_eq!(cookies.len(), 1);

        manager.delete_cookie(cookie_id).unwrap();
        let deleted = manager.get_cookie(cookie_id).unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_get_cookies_for_url() {
        let manager = CookieManager::new_in_memory().unwrap();

        let cookie1 = CookieCreate {
            name: "a".to_string(),
            value: "1".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: false,
            http_only: false,
            same_site: None,
            expires: None,
        };
        manager.create_cookie(cookie1).unwrap();

        let cookies = manager.get_cookies_for_url("https://example.com/path").unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name, "a");
    }
}
