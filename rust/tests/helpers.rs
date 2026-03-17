use rust_lib_omniread::http_client::HttpClient;
use std::fs;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;
use tokio::runtime::Runtime;

const TMP_DIR: &str = "tests/.tmp";
const CACHE_DURATION_MINUTES: u64 = 30;

pub fn get_cache_dir() -> String {
    TMP_DIR.to_string()
}

fn ensure_cache_dir() {
    let path = Path::new(TMP_DIR);
    if !path.exists() {
        fs::create_dir_all(path).unwrap();
    }
}

fn is_cache_valid(cache_path: &str) -> bool {
    let path = Path::new(cache_path);
    if !path.exists() {
        return false;
    }

    if let Ok(metadata) = fs::metadata(cache_path) {
        if let Ok(modified) = metadata.modified() {
            let cache_duration = Duration::from_secs(CACHE_DURATION_MINUTES * 60);
            if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                return elapsed < cache_duration;
            }
        }
    }
    false
}

pub fn download_and_cache(url: &str, cache_filename: &str) -> String {
    ensure_cache_dir();
    let cache_path = format!("{}/{}.html", TMP_DIR, cache_filename);

    if is_cache_valid(&cache_path) {
        return fs::read_to_string(&cache_path).unwrap();
    }

    let rt = Runtime::new().unwrap();
    let client = HttpClient::new().unwrap();
    let body = rt.block_on(client.get(url)).unwrap();

    fs::write(&cache_path, &body).unwrap();

    body
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_download_and_cache() {
        let url = "https://www.example.com";
        let cache_filename = "example";
        let body = download_and_cache(url, cache_filename);
        assert!(body.contains("Example"));
    }
}
