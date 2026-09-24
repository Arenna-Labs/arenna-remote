//! Arenna Remote integration glue: things that need the network or the
//! platform layer. Pure logic lives in `hbb_common::arenna`.

use hbb_common::{anyhow::anyhow, arenna, bail, ResultType};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

/// Timeouts for the update channel. The installer download gets a generous
/// one: reqwest's blocking client otherwise gives up after 30 s, which would
/// make updates impossible on slow links.
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
const SIGNATURE_TIMEOUT: Duration = Duration::from_secs(60);
const INSTALLER_TIMEOUT: Duration = Duration::from_secs(60 * 60);

fn user_agent() -> String {
    format!("{}/{}", arenna::APP_NAME, arenna::PRODUCT_VERSION)
}

/// Follows `latest_url` (GitHub answers `/releases/latest` with a redirect
/// to `/releases/tag/<tag>`, or to `/releases` when nothing is published)
/// and returns the release page when that version is newer than `current`.
/// No GitHub API call is involved, so there is no rate limit.
pub async fn newer_release_page(
    client: &reqwest::Client,
    latest_url: &str,
    current: &str,
) -> ResultType<Option<String>> {
    let resp = client
        .head(latest_url)
        .header(reqwest::header::USER_AGENT, user_agent())
        .timeout(CHECK_TIMEOUT)
        .send()
        .await?;
    let Some(version) = arenna::version_from_release_url(resp.url().as_str()) else {
        return Ok(None);
    };
    Ok(arenna::is_newer(&version, current).then(|| arenna::release_page_url(&version)))
}

fn asset_name(download_url: &str) -> ResultType<String> {
    download_url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("no file name in {download_url}"))
}

fn get(
    client: &reqwest::blocking::Client,
    url: &str,
    timeout: Duration,
) -> ResultType<reqwest::blocking::Response> {
    let resp = client
        .get(url)
        .header(reqwest::header::USER_AGENT, user_agent())
        .timeout(timeout)
        .send()?;
    if !resp.status().is_success() {
        bail!("cannot download {}: {}", url, resp.status());
    }
    Ok(resp)
}

/// Downloads an update installer and its `.sig` and verifies them in memory:
/// nothing is written to disk before the signature checks out. The
/// installer then runs elevated (as SYSTEM for automatic updates), so the
/// signature is the only thing trusted: not the TLS connection (upstream's
/// HTTP client may fall back to accepting invalid certificates) nor any file
/// left behind by an earlier run.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn download_verified_update(download_url: &str) -> ResultType<Vec<u8>> {
    download_verified_update_with_keys(download_url, arenna::UPDATE_PUBLIC_KEYS)
}

#[cfg_attr(not(windows), allow(dead_code))]
fn download_verified_update_with_keys(download_url: &str, public_keys: &[&str]) -> ResultType<Vec<u8>> {
    let asset = asset_name(download_url)?;
    let client = crate::hbbs_http::create_http_client_with_url(download_url);
    // The signature first: without one there is no point downloading.
    let signature = get(&client, &format!("{download_url}.sig"), SIGNATURE_TIMEOUT)?.text()?;
    let data = get(&client, download_url, INSTALLER_TIMEOUT)?.bytes()?.to_vec();
    arenna::verify_update_signature(&asset, &data, &signature, public_keys)?;
    Ok(data)
}

/// A verified installer on disk. While this value lives, the file is open
/// without write or delete sharing, so it cannot be replaced before the
/// process that runs it has started.
#[cfg_attr(not(windows), allow(dead_code))]
pub struct VerifiedUpdate {
    pub path: PathBuf,
    _lock: std::fs::File,
}

/// Stores a verified installer in `<install dir>\update` (inherits the
/// Program Files ACL: only administrators and SYSTEM can write there).
#[cfg(windows)]
pub fn save_verified_update(asset: &str, data: &[u8]) -> ResultType<VerifiedUpdate> {
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow!("no parent directory for {}", exe.display()))?
        .join("update");
    save_update_to(&dir, asset, data)
}

#[cfg_attr(not(windows), allow(dead_code))]
fn save_update_to(dir: &Path, asset: &str, data: &[u8]) -> ResultType<VerifiedUpdate> {
    std::fs::create_dir_all(dir)?;
    // Keep only the latest download; files still in use are left alone.
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            std::fs::remove_file(entry.path()).ok();
        }
    }
    let path = dir.join(asset);
    std::fs::write(&path, data)?;
    let mut lock = open_locked(&path)?;
    // Compare through the handle that now blocks writers: what gets started
    // is exactly what was verified.
    let mut on_disk = Vec::with_capacity(data.len());
    lock.read_to_end(&mut on_disk)?;
    if on_disk != data {
        bail!("{} changed on disk after verification", path.display());
    }
    Ok(VerifiedUpdate { path, _lock: lock })
}

#[cfg(windows)]
fn open_locked(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
}

#[cfg(not(windows))]
fn open_locked(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::File::open(path)
}

/// Command line that runs a downloaded installer in update mode. The path is
/// quoted: it lives under "C:\Program Files", and CreateProcess with an
/// unquoted path would first try to run "C:\Program.exe".
#[cfg_attr(not(windows), allow(dead_code))]
pub fn update_command_line(installer: &Path) -> String {
    format!("\"{}\" --update", installer.display())
}

/// Checks an installer downloaded by the UI (manual update, run through a
/// UAC prompt by the same user) against `<download_url>.sig`.
#[cfg(windows)]
pub fn verify_downloaded_update(path: &Path, download_url: &str) -> ResultType<()> {
    verify_downloaded_update_with_keys(path, download_url, arenna::UPDATE_PUBLIC_KEYS)
}

#[cfg_attr(not(windows), allow(dead_code))]
fn verify_downloaded_update_with_keys(
    path: &Path,
    download_url: &str,
    public_keys: &[&str],
) -> ResultType<()> {
    let asset = asset_name(download_url)?;
    let client = crate::hbbs_http::create_http_client_with_url(download_url);
    let signature = get(&client, &format!("{download_url}.sig"), SIGNATURE_TIMEOUT)?.text()?;
    let data = std::fs::read(path)?;
    arenna::verify_update_signature(&asset, &data, &signature, public_keys)
}

#[cfg(test)]
mod tests {
    use hbb_common::tokio;

    #[test]
    fn is_a_first_party_client_not_a_rustdesk_custom_client() {
        assert!(!crate::common::is_custom_client());
    }

    #[test]
    fn display_name_differs_from_internal_name() {
        assert_eq!(crate::common::get_app_display_name(), "Arenna Remote");
        assert_eq!(crate::common::get_app_name(), "ArennaRemote");
    }

    #[test]
    fn translations_show_the_display_name() {
        assert_eq!(
            crate::lang::translate_locale("About RustDesk".to_owned(), "en"),
            "About Arenna Remote"
        );
        assert_eq!(
            crate::lang::translate_locale("About RustDesk".to_owned(), "es"),
            "Acerca de Arenna Remote"
        );
    }

    #[test]
    fn the_default_server_is_not_rustdesks_public_server() {
        // Upstream shows "set up your own server" tips and applies public
        // server limits when this is true.
        assert!(!crate::common::using_public_server());
    }

    #[test]
    fn never_talks_to_an_api_server() {
        assert_eq!(crate::common::get_api_server(String::new(), String::new()), "");
    }

    /// Minimal HTTP/1.1 server for update-channel tests. `routes` maps a
    /// request path to a raw response (status line + headers + body).
    fn serve(routes: Vec<(&'static str, String)>) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("").to_string();
                let resp = routes
                    .iter()
                    .find(|(p, _)| *p == path)
                    .map(|(_, r)| r.clone())
                    .unwrap_or_else(|| {
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .to_string()
                    });
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        format!("http://{addr}")
    }

    fn redirect(to: &str) -> String {
        format!("HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
    }

    fn ok(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    }

    #[tokio::test]
    async fn finds_a_newer_release_through_the_latest_redirect() {
        let base = serve(vec![
            ("/latest", redirect("/o/r/releases/tag/v9.9.9")),
            ("/o/r/releases/tag/v9.9.9", ok("")),
        ]);
        let client = reqwest::Client::new();
        let found = super::newer_release_page(&client, &format!("{base}/latest"), "1.0.0")
            .await
            .unwrap();
        assert_eq!(found, Some(hbb_common::arenna::release_page_url("9.9.9")));
    }

    #[tokio::test]
    async fn same_or_older_release_is_not_an_update() {
        let base = serve(vec![
            ("/latest", redirect("/o/r/releases/tag/v1.0.0")),
            ("/o/r/releases/tag/v1.0.0", ok("")),
        ]);
        let client = reqwest::Client::new();
        let url = format!("{base}/latest");
        assert_eq!(super::newer_release_page(&client, &url, "1.0.0").await.unwrap(), None);
        assert_eq!(super::newer_release_page(&client, &url, "2.0.0").await.unwrap(), None);
    }

    #[tokio::test]
    async fn no_published_release_is_not_an_update() {
        // GitHub redirects /releases/latest to /releases when there is none.
        let base = serve(vec![
            ("/latest", redirect("/o/r/releases")),
            ("/o/r/releases", ok("")),
        ]);
        let client = reqwest::Client::new();
        let found = super::newer_release_page(&client, &format!("{base}/latest"), "0.0.0")
            .await
            .unwrap();
        assert_eq!(found, None);
    }

    const ASSET: &str = "arenna-remote-9.9.9-x86_64.exe";

    fn keypair() -> (String, hbb_common::sodiumoxide::crypto::sign::SecretKey) {
        use hbb_common::sodiumoxide::{base64::{encode, Variant}, crypto::sign};
        let (pk, sk) = sign::gen_keypair();
        (encode(pk.0, Variant::Original), sk)
    }

    fn sig_line(data: &[u8], sk: &hbb_common::sodiumoxide::crypto::sign::SecretKey) -> String {
        use hbb_common::sodiumoxide::{base64::{encode, Variant}, crypto::sign};
        let message = hbb_common::arenna::signed_message(ASSET, data);
        encode(sign::sign_detached(&message, sk).to_bytes(), Variant::Original)
    }

    fn temp_file(content: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "arenna-test-{}-{}",
            std::process::id(),
            hbb_common::rand::random::<u32>()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(ASSET);
        std::fs::write(&path, content).unwrap();
        path
    }

    fn ok_bytes(body: &[u8]) -> String {
        // Test payloads are ASCII.
        ok(std::str::from_utf8(body).unwrap())
    }

    #[test]
    fn downloads_and_verifies_an_update_in_memory() {
        let (pk, sk) = keypair();
        let base = serve(vec![
            ("/v9/arenna-remote-9.9.9-x86_64.exe", ok_bytes(b"installer")),
            ("/v9/arenna-remote-9.9.9-x86_64.exe.sig", ok(&format!("{}\n", sig_line(b"installer", &sk)))),
        ]);
        let url = format!("{base}/v9/{ASSET}");
        let data = super::download_verified_update_with_keys(&url, &[&pk]).unwrap();
        assert_eq!(data, b"installer");
    }

    #[test]
    fn rejects_a_tampered_update_download() {
        let (pk, sk) = keypair();
        let base = serve(vec![
            ("/v9/arenna-remote-9.9.9-x86_64.exe", ok_bytes(b"installer + malware")),
            ("/v9/arenna-remote-9.9.9-x86_64.exe.sig", ok(&sig_line(b"installer", &sk))),
        ]);
        let url = format!("{base}/v9/{ASSET}");
        assert!(super::download_verified_update_with_keys(&url, &[&pk]).is_err());
    }

    #[test]
    fn rejects_an_update_without_signature() {
        let (pk, _) = keypair();
        let base = serve(vec![("/v9/arenna-remote-9.9.9-x86_64.exe", ok_bytes(b"installer"))]);
        let url = format!("{base}/v9/{ASSET}");
        assert!(super::download_verified_update_with_keys(&url, &[&pk]).is_err());
    }

    #[test]
    fn update_command_line_quotes_the_installer_path() {
        let path = std::path::Path::new(r"C:\Program Files\ArennaRemote\update\arenna-remote-9.9.9-x86_64.exe");
        assert_eq!(
            super::update_command_line(path),
            r#""C:\Program Files\ArennaRemote\update\arenna-remote-9.9.9-x86_64.exe" --update"#
        );
    }

    #[test]
    fn saves_the_verified_update_replacing_older_downloads() {
        let dir = temp_file(b"").parent().unwrap().join("update");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("arenna-remote-9.9.8-x86_64.exe"), b"old").unwrap();
        let saved = super::save_update_to(&dir, ASSET, b"verified installer").unwrap();
        assert_eq!(saved.path, dir.join(ASSET));
        assert_eq!(std::fs::read(&saved.path).unwrap(), b"verified installer");
        assert!(!dir.join("arenna-remote-9.9.8-x86_64.exe").exists());
        drop(saved);
        std::fs::remove_dir_all(dir.parent().unwrap()).ok();
    }

    #[test]
    fn accepts_a_manual_download_whose_signature_matches() {
        let (pk, sk) = keypair();
        let path = temp_file(b"installer");
        let base = serve(vec![("/v9/arenna-remote-9.9.9-x86_64.exe.sig", ok(&sig_line(b"installer", &sk)))]);
        let res = super::verify_downloaded_update_with_keys(&path, &format!("{base}/v9/{ASSET}"), &[&pk]);
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
        assert!(res.is_ok(), "{res:?}");
    }

    #[test]
    fn rejects_a_manual_download_that_was_tampered_with() {
        let (pk, sk) = keypair();
        let path = temp_file(b"installer + malware");
        let base = serve(vec![("/v9/arenna-remote-9.9.9-x86_64.exe.sig", ok(&sig_line(b"installer", &sk)))]);
        let res = super::verify_downloaded_update_with_keys(&path, &format!("{base}/v9/{ASSET}"), &[&pk]);
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
        assert!(res.is_err());
    }
}
