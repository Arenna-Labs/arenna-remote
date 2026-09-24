fn main() {
    #[cfg(windows)]
    {
        use std::io::Write;
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../res/icon.ico")
            .set_language(winapi::um::winnt::MAKELANGID(
                winapi::um::winnt::LANG_ENGLISH,
                winapi::um::winnt::SUBLANG_ENGLISH_US,
            ))
            .set_manifest_file("../../res/manifest.xml");
        // Arenna Remote: the installer shows the product version (set by CI
        // from the release tag), not the RustDesk version of this crate.
        println!("cargo:rerun-if-env-changed=ARENNA_VERSION");
        if let Ok(version) = std::env::var("ARENNA_VERSION") {
            let packed = version
                .split('.')
                .map(|part| part.parse::<u64>().unwrap_or(0))
                .chain(std::iter::repeat(0))
                .take(4)
                .fold(0u64, |acc, part| (acc << 16) | (part & 0xffff));
            res.set("FileVersion", &version)
                .set("ProductVersion", &version)
                .set_version_info(winres::VersionInfo::FILEVERSION, packed)
                .set_version_info(winres::VersionInfo::PRODUCTVERSION, packed);
        }
        match res.compile() {
            Err(e) => {
                write!(std::io::stderr(), "{}", e).unwrap();
                std::process::exit(1);
            }
            Ok(_) => {}
        }
    }
}
