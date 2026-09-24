//! Arenna Remote integration glue: things that need the network or the
//! platform layer. Pure logic lives in `hbb_common::arenna`.

use hbb_common::{arenna, bail, ResultType};
use std::path::Path;

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
        .send()
        .await?;
    let Some(version) = arenna::version_from_release_url(resp.url().as_str()) else {
        return Ok(None);
    };
    Ok(arenna::is_newer(&version, current).then(|| arenna::release_page_url(&version)))
}

/// Refuses a downloaded installer unless `<download_url>.sig` is a valid
/// signature of it by the release pipeline. The installer runs elevated (as
/// SYSTEM for automatic updates), so nothing else is trusted: not the TLS
/// connection (upstream's HTTP client may accept invalid certificates) nor
/// the file name.
#[cfg(windows)]
pub fn verify_downloaded_update(path: &Path, download_url: &str) -> ResultType<()> {
    verify_downloaded_update_with_key(path, download_url, arenna::UPDATE_PUBLIC_KEY)
}

#[cfg_attr(not(windows), allow(dead_code))]
fn verify_downloaded_update_with_key(
    path: &Path,
    download_url: &str,
    public_key: &str,
) -> ResultType<()> {
    let sig_url = format!("{download_url}.sig");
    let client = crate::hbbs_http::create_http_client_with_url(&sig_url);
    let resp = client
        .get(&sig_url)
        .header(reqwest::header::USER_AGENT, user_agent())
        .send()?;
    if !resp.status().is_success() {
        bail!("cannot download {}: {}", sig_url, resp.status());
    }
    let signature = resp.text()?;
    let data = std::fs::read(path)?;
    arenna::verify_update_signature(&data, &signature, public_key)
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

    fn signed_file(content: &[u8]) -> (std::path::PathBuf, String, String) {
        use hbb_common::sodiumoxide::{
            base64::{encode, Variant},
            crypto::sign,
        };
        let (pk, sk) = sign::gen_keypair();
        let sig = sign::sign_detached(content, &sk);
        let path = std::env::temp_dir().join(format!(
            "arenna-remote-test-{}-{}.exe",
            std::process::id(),
            hbb_common::rand::random::<u32>()
        ));
        std::fs::write(&path, content).unwrap();
        (
            path,
            encode(sig.to_bytes(), Variant::Original),
            encode(pk.0, Variant::Original),
        )
    }

    #[test]
    fn accepts_a_download_whose_signature_matches() {
        let (path, sig, pk) = signed_file(b"installer");
        let base = serve(vec![("/v1/a.exe.sig", ok(&format!("{sig}\n")))]);
        let res = super::verify_downloaded_update_with_key(&path, &format!("{base}/v1/a.exe"), &pk);
        std::fs::remove_file(&path).ok();
        assert!(res.is_ok(), "{res:?}");
    }

    #[test]
    fn rejects_a_download_that_was_tampered_with() {
        let (path, sig, pk) = signed_file(b"installer");
        std::fs::write(&path, b"installer + malware").unwrap();
        let base = serve(vec![("/v1/a.exe.sig", ok(&sig))]);
        let res = super::verify_downloaded_update_with_key(&path, &format!("{base}/v1/a.exe"), &pk);
        std::fs::remove_file(&path).ok();
        assert!(res.is_err());
    }

    #[test]
    fn rejects_a_download_without_signature() {
        let (path, _sig, pk) = signed_file(b"installer");
        let base = serve(vec![]);
        let res = super::verify_downloaded_update_with_key(&path, &format!("{base}/v1/a.exe"), &pk);
        std::fs::remove_file(&path).ok();
        assert!(res.is_err());
    }
}
