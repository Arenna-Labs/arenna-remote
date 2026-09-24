//! Arenna Remote product identity and update-channel helpers.
//!
//! Everything that makes this fork "Arenna Remote" rather than RustDesk and
//! that does not need the network lives here, so upstream merges only touch
//! a handful of call sites.

use std::collections::HashMap;

/// Internal application name: config directory, Windows service and
/// executable, IPC pipe, URI scheme. Must not contain spaces.
pub const APP_NAME: &str = "ArennaRemote";
/// Name shown to people (window titles, shortcuts, translations).
pub const DISPLAY_NAME: &str = "Arenna Remote";
pub const COMPANY: &str = "Arenna Labs S.L.";
pub const WEBSITE: &str = "https://arennalabs.com";
/// GitHub repository whose releases are the update channel.
pub const RELEASES_REPO: &str = "Arenna-Labs/arenna_support_app";

/// Self-hosted rustdesk-server (hbbs/hbbr) used by default.
pub const SERVER_HOST: &str = "rustdesk.arenna38.com";
/// Base64 of the server's `id_ed25519.pub`.
pub const SERVER_PUBLIC_KEY: &str = "t4yov1rUxoLGLbxyT7CgDaIEqfqIRMBWFBdLIMBaTJE=";

/// Ed25519 public key that verifies the `.sig` of every Windows update.
/// The private half is the `UPDATE_SIGNING_KEY` GitHub Actions secret.
pub const UPDATE_PUBLIC_KEY: &str = "qFMXX9S2xKnWwR8CwFS8cFXvR7SaiV3vQc1rBewKcZE=";

/// Product version (`X.Y.Z` from the release tag), injected by CI through the
/// `ARENNA_VERSION` environment variable. Unlike `crate::VERSION` (the
/// RustDesk protocol version peers use to enable features) it is only used
/// for display and for the update check.
pub const PRODUCT_VERSION: &str = match option_env!("ARENNA_VERSION") {
    Some(v) => v,
    None => "0.0.0",
};

fn map(entries: &[(&str, &str)]) -> HashMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Initial `config::BUILTIN_SETTINGS`.
pub fn builtin_settings() -> HashMap<String, String> {
    map(&[
        // The server is fixed at build time; clients should not change it.
        ("hide-server-settings", "Y"),
        // OSS hbbs has no HTTP API: never send heartbeats/sysinfo anywhere.
        ("register-device", "N"),
        // The remote printer driver only works with RustDesk-signed binaries.
        ("hide-remote-printer-settings", "Y"),
    ])
}

/// Initial `config::DEFAULT_SETTINGS` (values used while unset).
pub fn default_settings() -> HashMap<String, String> {
    map(&[("allow-auto-update", "Y")])
}

/// Initial `config::HARD_SETTINGS`.
pub fn hard_settings() -> HashMap<String, String> {
    // Account login needs the Pro API server.
    map(&[("disable-account", "Y")])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_arenna_remote() {
        assert_eq!(*crate::config::APP_NAME.read().unwrap(), "ArennaRemote");
        assert_eq!(DISPLAY_NAME, "Arenna Remote");
        assert_eq!(COMPANY, "Arenna Labs S.L.");
    }

    #[test]
    fn default_server_is_the_arenna_server() {
        assert_eq!(crate::config::RENDEZVOUS_SERVERS, &["rustdesk.arenna38.com"]);
        assert_eq!(crate::config::RS_PUB_KEY, SERVER_PUBLIC_KEY);
    }

    #[test]
    fn builtin_settings_hide_server_settings_and_skip_api() {
        let builtin = crate::config::BUILTIN_SETTINGS.read().unwrap();
        assert_eq!(builtin.get("hide-server-settings").map(String::as_str), Some("Y"));
        assert_eq!(builtin.get("register-device").map(String::as_str), Some("N"));
        assert_eq!(
            builtin.get("hide-remote-printer-settings").map(String::as_str),
            Some("Y")
        );
        drop(builtin);
        assert!(crate::config::Config::no_register_device());
    }

    #[test]
    fn account_login_is_disabled() {
        assert!(crate::config::is_disable_account());
    }

    #[test]
    fn auto_update_is_on_by_default() {
        let defaults = crate::config::DEFAULT_SETTINGS.read().unwrap();
        assert_eq!(defaults.get("allow-auto-update").map(String::as_str), Some("Y"));
    }
}
