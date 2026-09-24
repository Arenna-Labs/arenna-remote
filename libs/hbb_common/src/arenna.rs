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
pub const RELEASES_REPO: &str = "Arenna-Labs/arenna-remote";

/// Self-hosted rustdesk-server (hbbs/hbbr) used by default.
pub const SERVER_HOST: &str = "rustdesk.arenna38.com";
/// Base64 of the server's `id_ed25519.pub`.
pub const SERVER_PUBLIC_KEY: &str = "7c6bMZlmhqyWFvA3sFQDtQKQr29M+Z2CsjbYMLJdQ8g=";

/// Ed25519 public keys trusted to sign Windows updates (`<asset>.sig`). The
/// private halves live in the `UPDATE_SIGNING_KEY` GitHub Actions secret.
/// To rotate: add the new key here and sign releases with both keys until
/// every client runs a version that trusts the new one.
pub const UPDATE_PUBLIC_KEYS: &[&str] = &["qFMXX9S2xKnWwR8CwFS8cFXvR7SaiV3vQc1rBewKcZE="];

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

/// Bytes covered by an update signature: the asset file name, a newline,
/// then the file. The name carries the version, so an older signed
/// installer cannot be passed off as a newer one.
pub fn signed_message(asset_name: &str, data: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(asset_name.len() + 1 + data.len());
    message.extend_from_slice(asset_name.as_bytes());
    message.push(b'\n');
    message.extend_from_slice(data);
    message
}

/// Checks `sig_file` (one base64 Ed25519 signature per line; several during
/// a key rotation) for the asset `asset_name` with content `data`. Valid if
/// any line verifies with any of `public_keys`.
pub fn verify_update_signature(
    asset_name: &str,
    data: &[u8],
    sig_file: &str,
    public_keys: &[&str],
) -> crate::ResultType<()> {
    use sodiumoxide::base64::{decode, Variant};
    use sodiumoxide::crypto::sign;

    let keys: Vec<sign::PublicKey> = public_keys
        .iter()
        .filter_map(|k| decode(k.trim(), Variant::Original).ok())
        .filter_map(|raw| sign::PublicKey::from_slice(&raw))
        .collect();
    let signatures: Vec<sign::Signature> = sig_file
        .split_whitespace()
        .filter_map(|line| decode(line, Variant::Original).ok())
        .filter_map(|raw| sign::Signature::from_bytes(&raw).ok())
        .collect();
    if keys.is_empty() {
        crate::bail!("no valid update public key");
    }
    if signatures.is_empty() {
        crate::bail!("no valid signature in the .sig file");
    }
    let message = signed_message(asset_name, data);
    let valid = signatures
        .iter()
        .any(|sig| keys.iter().any(|pk| sign::verify_detached(sig, &message, pk)));
    if !valid {
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
        let base = "https://github.com/Arenna-Labs/arenna-remote/releases";
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
            "https://github.com/Arenna-Labs/arenna-remote/releases/download/v1.2.3/arenna-remote-1.2.3-x86_64.exe"
        );
        assert_eq!(
            release_page_url("1.2.3"),
            "https://github.com/Arenna-Labs/arenna-remote/releases/tag/v1.2.3"
        );
        assert!(is_update_file_name("arenna-remote-1.2.3-x86_64.exe"));
        assert!(!is_update_file_name("rustdesk-1.4.9-x86_64.exe"));
        assert!(!is_update_file_name("arenna-remote-notes.txt"));
    }

    fn test_key() -> (String, sodiumoxide::crypto::sign::SecretKey) {
        use sodiumoxide::base64::{encode, Variant};
        let (pk, sk) = sodiumoxide::crypto::sign::gen_keypair();
        (encode(pk.0, Variant::Original), sk)
    }

    fn sign_line(asset: &str, data: &[u8], sk: &sodiumoxide::crypto::sign::SecretKey) -> String {
        use sodiumoxide::base64::{encode, Variant};
        let sig = sodiumoxide::crypto::sign::sign_detached(&signed_message(asset, data), sk);
        encode(sig.to_bytes(), Variant::Original)
    }

    const ASSET: &str = "arenna-remote-1.2.3-x86_64.exe";

    #[test]
    fn accepts_a_valid_signature() {
        let (pk, sk) = test_key();
        let sig = sign_line(ASSET, b"installer", &sk);
        assert!(verify_update_signature(ASSET, b"installer", &sig, &[&pk]).is_ok());
        // Trailing newline as written by .github/scripts/sign_update.py.
        let sig_file = format!("{sig}\n");
        assert!(verify_update_signature(ASSET, b"installer", &sig_file, &[&pk]).is_ok());
    }

    #[test]
    fn rejects_tampered_data_other_keys_and_garbage() {
        let (pk, sk) = test_key();
        let (other_pk, _) = test_key();
        let sig = sign_line(ASSET, b"installer", &sk);
        assert!(verify_update_signature(ASSET, b"tampered", &sig, &[&pk]).is_err());
        assert!(verify_update_signature(ASSET, b"installer", &sig, &[&other_pk]).is_err());
        assert!(verify_update_signature(ASSET, b"installer", &sig, &[]).is_err());
        assert!(verify_update_signature(ASSET, b"installer", "", &[&pk]).is_err());
        assert!(verify_update_signature(ASSET, b"installer", "not base64!", &[&pk]).is_err());
        assert!(verify_update_signature(ASSET, b"installer", &pk, &[&pk]).is_err());
    }

    #[test]
    fn signature_is_bound_to_the_asset_name() {
        // An older signed installer must not be accepted under a newer name.
        let (pk, sk) = test_key();
        let old = sign_line("arenna-remote-1.0.0-x86_64.exe", b"old installer", &sk);
        assert!(verify_update_signature(ASSET, b"old installer", &old, &[&pk]).is_err());
        // A plain signature over the bytes alone is not enough either.
        use sodiumoxide::base64::{encode, Variant};
        let bare = sodiumoxide::crypto::sign::sign_detached(b"installer", &sk);
        let bare = encode(bare.to_bytes(), Variant::Original);
        assert!(verify_update_signature(ASSET, b"installer", &bare, &[&pk]).is_err());
    }

    #[test]
    fn supports_key_rotation_with_several_lines_and_keys() {
        let (old_pk, old_sk) = test_key();
        let (new_pk, new_sk) = test_key();
        let both = format!(
            "{}\n{}\n",
            sign_line(ASSET, b"installer", &old_sk),
            sign_line(ASSET, b"installer", &new_sk)
        );
        // Old clients (old key only) and new clients (new key only) accept it.
        assert!(verify_update_signature(ASSET, b"installer", &both, &[&old_pk]).is_ok());
        assert!(verify_update_signature(ASSET, b"installer", &both, &[&new_pk]).is_ok());
        // A garbage line does not prevent a later valid one from matching.
        let noisy = format!("garbage\n{}", sign_line(ASSET, b"installer", &new_sk));
        assert!(verify_update_signature(ASSET, b"installer", &noisy, &[&old_pk, &new_pk]).is_ok());
    }

    #[test]
    fn update_public_keys_are_valid_ed25519_keys() {
        use sodiumoxide::base64::{decode, Variant};
        assert!(!UPDATE_PUBLIC_KEYS.is_empty());
        for key in UPDATE_PUBLIC_KEYS {
            let raw = decode(key, Variant::Original).unwrap();
            assert!(sodiumoxide::crypto::sign::PublicKey::from_slice(&raw).is_some());
        }
    }

    #[test]
    fn accepts_signatures_made_by_the_ci_signing_script() {
        // .github/scripts/sign_update.py (Python `cryptography`) with the
        // test-only seeds 00..1f and 20..3f, file arenna-remote-1.2.3-x86_64.exe
        // containing b"Arenna Remote update".
        let key1 = "A6EHv/POEL4dcN0Y50vAmWfk1jCbpQ1fHdyGZBJVMbg=";
        let key2 = "Kay64UG8yvCyLhqU000LxzYeUm0L/hLIl5S8kyKWbdc=";
        let sig_file = "aygNdkF1VMZWNbEEWNA2v0SCdmxcXByE+haaZ+UKWyvOxxvRhNKeuJptf5AHl58tBY019AVj29CZjSxsi2QqDA==\nG2Y3MIHqg+UfMLqQbIJo3+awjxLb6gwQFL+QKye2Skt7iJAnb3AwnSGSNy1X9aXmIFERLbW4gQq6c9MG23GRCw==\n";
        let data = b"Arenna Remote update";
        assert!(verify_update_signature(ASSET, data, sig_file, &[key1]).is_ok());
        assert!(verify_update_signature(ASSET, data, sig_file, &[key2]).is_ok());
        assert!(verify_update_signature(ASSET, b"Arenna Remote update!", sig_file, &[key1, key2]).is_err());
    }
}
