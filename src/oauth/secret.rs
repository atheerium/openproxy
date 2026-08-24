//! OAuth client-secret resolution.
//!
//! Client secrets for OAuth providers are resolved **exclusively** from
//! environment variables at runtime. No hardcoded fallback values are stored
//! in the repository. Operators MUST set the appropriate environment variables
//! before starting the server, otherwise OAuth flows for those providers will
//! not be available.
//!
//! | Provider     | Environment variable            |
//! |--------------|---------------------------------|
//! | iFlow        | `IFLOW_CLIENT_SECRET`           |
//! | Qoder        | `QODER_CLIENT_SECRET`           |
//! | Antigravity  | `ANTIGRAVITY_CLIENT_SECRET`     |
//! | Gemini       | `GEMINI_CLIENT_SECRET`          |

use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Resolution cache: resolved once per name, cached for the process lifetime.
static CACHE: Lazy<Mutex<std::collections::HashMap<String, &'static str>>> =
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

fn resolve(name: &str) -> &'static str {
    let mut cache = CACHE.lock().unwrap();
    if let Some(v) = cache.get(name) {
        return *v;
    }
    let leaked: &'static str = Box::leak(std::env::var(name).unwrap_or_default().into_boxed_str());
    cache.insert(name.to_string(), leaked);
    leaked
}

#[cfg(test)]
fn clear_cache() {
    let mut cache = CACHE.lock().unwrap();
    cache.clear();
}

/// iFlow OAuth client secret (env `IFLOW_CLIENT_SECRET`).
pub fn iflow_client_secret() -> &'static str {
    resolve("IFLOW_CLIENT_SECRET")
}

/// Qoder OAuth client secret (env `QODER_CLIENT_SECRET`).
pub fn qoder_client_secret() -> &'static str {
    resolve("QODER_CLIENT_SECRET")
}

/// Antigravity OAuth client secret (env `ANTIGRAVITY_CLIENT_SECRET`).
pub fn antigravity_client_secret() -> &'static str {
    resolve("ANTIGRAVITY_CLIENT_SECRET")
}

/// Gemini CLI OAuth client secret (env `GEMINI_CLIENT_SECRET`).
pub fn gemini_cli_client_secret() -> &'static str {
    resolve("GEMINI_CLIENT_SECRET")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialize tests that mutate global environment variables and the secret
    /// cache to avoid cross-test interference when the lib test suite runs in
    /// parallel.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn secrets_return_empty_without_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_cache();
        std::env::remove_var("IFLOW_CLIENT_SECRET");
        std::env::remove_var("QODER_CLIENT_SECRET");
        std::env::remove_var("ANTIGRAVITY_CLIENT_SECRET");
        std::env::remove_var("GEMINI_CLIENT_SECRET");
        // Without env vars set, secrets are empty strings (no hardcoded fallback).
        assert_eq!(iflow_client_secret(), "");
        assert_eq!(qoder_client_secret(), "");
        assert_eq!(antigravity_client_secret(), "");
        assert_eq!(gemini_cli_client_secret(), "");
    }

    #[test]
    fn secrets_resolve_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_cache();
        std::env::set_var("IFLOW_CLIENT_SECRET", "test-secret-iflow");
        std::env::set_var("QODER_CLIENT_SECRET", "test-secret-qoder");
        std::env::set_var("ANTIGRAVITY_CLIENT_SECRET", "test-secret-anti");
        std::env::set_var("GEMINI_CLIENT_SECRET", "test-secret-gemini");
        assert_eq!(iflow_client_secret(), "test-secret-iflow");
        assert_eq!(qoder_client_secret(), "test-secret-qoder");
        assert_eq!(antigravity_client_secret(), "test-secret-anti");
        assert_eq!(gemini_cli_client_secret(), "test-secret-gemini");
        std::env::remove_var("IFLOW_CLIENT_SECRET");
        std::env::remove_var("QODER_CLIENT_SECRET");
        std::env::remove_var("ANTIGRAVITY_CLIENT_SECRET");
        std::env::remove_var("GEMINI_CLIENT_SECRET");
    }
}
