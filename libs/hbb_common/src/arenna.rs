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
pub const SERVER_PUBLIC_KEY: &str = "7c6bMZlmhqyWFvA3sFQDtQKQr29M+Z2CsjbYMLJdQ8g=";

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


/// Prefix of every release asset (`arenna-remote-X.Y.Z-...`).
const ASSET_PREFIX: &str = "arenna-remote-";

/// Redirects to `/releases/tag/<latest tag>` (or to `/releases` when nothing
/// is published). Following the redirect needs no API call and no token, so
/// it is not subject to the GitHub API rate limit.
pub fn latest_release_url() -> String {
    format!("https://github.com/{RELEASES_REPO}/releases/latest")
}

pub fn release_page_url(version: &str) -> String {
    format!("https://github.com/{RELEASES_REPO}/releases/tag/v{version}")
}

pub fn release_download_url(version: &str, asset: &str) -> String {
    format!("https://github.com/{RELEASES_REPO}/releases/download/v{version}/{asset}")
}

/// Self-extracting Windows installer published for `version`.
pub fn windows_asset_name(version: &str, arch: &str) -> String {
    format!("{ASSET_PREFIX}{version}-{arch}.exe")
}

/// Whether a downloaded file looks like one of our Windows installers.
pub fn is_update_file_name(name: &str) -> bool {
    name.starts_with(ASSET_PREFIX) && name.ends_with(".exe")
}

/// `v1.2.3` / `V1.2.3` / `1.2.3` -> `1.2.3`. Anything that is not
/// dot-separated numbers (e.g. `nightly`) is rejected.
pub fn tag_version(tag: &str) -> Option<String> {
    let version = tag
        .strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag);
    let valid = !version.is_empty()
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
    valid.then(|| version.to_owned())
}

/// Version of a `.../releases/tag/<tag>` URL, `None` for any other URL.
pub fn version_from_release_url(url: &str) -> Option<String> {
    let (rest, tag) = url.trim_end_matches('/').rsplit_once('/')?;
    if !rest.ends_with("/releases/tag") {
        return None;
    }
    tag_version(tag)
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    crate::get_version_number(candidate) > crate::get_version_number(current)
}

/// Checks a detached Ed25519 signature (base64) of `data`.
pub fn verify_update_signature(data: &[u8], sig_b64: &str, pk_b64: &str) -> crate::ResultType<()> {
    use sodiumoxide::base64::{decode, Variant};
    use sodiumoxide::crypto::sign;

    let sig = decode(sig_b64.trim(), Variant::Original)
        .map_err(|_| anyhow::anyhow!("update signature is not valid base64"))?;
    let sig = sign::Signature::from_bytes(&sig)
        .map_err(|_| anyhow::anyhow!("update signature has the wrong length"))?;
    let pk = decode(pk_b64.trim(), Variant::Original)
        .map_err(|_| anyhow::anyhow!("update public key is not valid base64"))?;
    let pk = sign::PublicKey::from_slice(&pk)
        .ok_or_else(|| anyhow::anyhow!("update public key has the wrong length"))?;
    if !sign::verify_detached(&sig, data, &pk) {
        crate::bail!("update signature does not match");
    }
    Ok(())
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
    fn server_public_key_is_the_production_hbbs_key() {
        // /opt/rustdesk-server/data/id_ed25519.pub on rustdesk.arenna38.com
        assert_eq!(SERVER_PUBLIC_KEY, "7c6bMZlmhqyWFvA3sFQDtQKQr29M+Z2CsjbYMLJdQ8g=");
        let raw = sodiumoxide::base64::decode(SERVER_PUBLIC_KEY, sodiumoxide::base64::Variant::Original).unwrap();
        assert!(sodiumoxide::crypto::sign::PublicKey::from_slice(&raw).is_some());
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

    #[test]
    fn parses_release_tag_urls() {
        let base = "https://github.com/Arenna-Labs/arenna_support_app/releases";
        assert_eq!(version_from_release_url(&format!("{base}/tag/v1.2.3")), Some("1.2.3".into()));
        assert_eq!(version_from_release_url(&format!("{base}/tag/V2.0.0")), Some("2.0.0".into()));
        assert_eq!(version_from_release_url(&format!("{base}/tag/1.0.1")), Some("1.0.1".into()));
        assert_eq!(version_from_release_url(&format!("{base}/tag/v1.0.1/")), Some("1.0.1".into()));
        // No release published yet: GitHub redirects /releases/latest to /releases.
        assert_eq!(version_from_release_url(base), None);
        assert_eq!(version_from_release_url(&format!("{base}/tag/vnext")), None);
        assert_eq!(version_from_release_url(&format!("{base}/tag/v1..2")), None);
        assert_eq!(version_from_release_url(&format!("{base}/tag/")), None);
        assert_eq!(version_from_release_url(""), None);
        assert_eq!(version_from_release_url("https://github.com/login"), None);
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(is_newer("1.1.0", "1.0.9"));
        assert!(is_newer("2.0.0", "1.99.99"));
        assert!(is_newer("1.0.0", "0.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.0", "1.0.0"));
    }

    #[test]
    fn tag_version_accepts_optional_v_prefix() {
        assert_eq!(tag_version("v1.2.3"), Some("1.2.3".into()));
        assert_eq!(tag_version("1.2.3"), Some("1.2.3".into()));
        assert_eq!(tag_version("nightly"), None);
    }

    #[test]
    fn builds_windows_asset_names_and_urls() {
        assert_eq!(windows_asset_name("1.2.3", "x86_64"), "arenna-remote-1.2.3-x86_64.exe");
        assert_eq!(
            release_download_url("1.2.3", "arenna-remote-1.2.3-x86_64.exe"),
            "https://github.com/Arenna-Labs/arenna_support_app/releases/download/v1.2.3/arenna-remote-1.2.3-x86_64.exe"
        );
        assert_eq!(
            release_page_url("1.2.3"),
            "https://github.com/Arenna-Labs/arenna_support_app/releases/tag/v1.2.3"
        );
        assert!(is_update_file_name("arenna-remote-1.2.3-x86_64.exe"));
        assert!(!is_update_file_name("rustdesk-1.4.9-x86_64.exe"));
        assert!(!is_update_file_name("arenna-remote-notes.txt"));
    }

    #[test]
    fn verifies_update_signatures() {
        use sodiumoxide::base64::{encode, Variant};
        use sodiumoxide::crypto::sign;
        let (pk, sk) = sign::gen_keypair();
        let data = b"installer bytes";
        let sig = sign::sign_detached(data, &sk);
        let pk_b64 = encode(pk.0, Variant::Original);
        let sig_b64 = encode(sig.to_bytes(), Variant::Original);

        assert!(verify_update_signature(data, &sig_b64, &pk_b64).is_ok());
        // Trailing newline as written by .github/scripts/sign_update.py.
        assert!(verify_update_signature(data, &format!("{sig_b64}\n"), &pk_b64).is_ok());
        assert!(verify_update_signature(b"tampered", &sig_b64, &pk_b64).is_err());
        assert!(verify_update_signature(data, "", &pk_b64).is_err());
        assert!(verify_update_signature(data, "not base64!", &pk_b64).is_err());
        assert!(verify_update_signature(data, &pk_b64, &pk_b64).is_err());
        let (other_pk, _) = sign::gen_keypair();
        let other_b64 = encode(other_pk.0, Variant::Original);
        assert!(verify_update_signature(data, &sig_b64, &other_b64).is_err());
    }

    #[test]
    fn update_public_key_is_a_valid_ed25519_key() {
        use sodiumoxide::base64::{decode, Variant};
        let raw = decode(UPDATE_PUBLIC_KEY, Variant::Original).unwrap();
        assert!(sodiumoxide::crypto::sign::PublicKey::from_slice(&raw).is_some());
    }

    #[test]
    fn accepts_signatures_made_by_the_ci_signing_script() {
        // Produced by .github/scripts/sign_update.py (Python `cryptography`)
        // with the test-only seed 00 01 02 .. 1f over b"Arenna Remote update".
        let pk = "A6EHv/POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbg=";
        let sig = "jaOXeyvvXSV5iIJVJtaGKuN9mdVpiRd1ue/7JVghgO1nwlmkPpmaptpFF388fkNU/G1yL5FnVGWQai3/zm+cAg==";
        assert!(verify_update_signature(b"Arenna Remote update", sig, pk).is_ok());
        assert!(verify_update_signature(b"Arenna Remote update!", sig, pk).is_err());
    }
}
