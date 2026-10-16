//! See if a GitHub release is newer than this build, and check its checksum.
//!
//! The only network call is the GitHub releases API, plus the setup file and
//! `SHA256SUMS.txt` from that release. A failed check leaves the app running.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const RELEASES_URL: &str = "https://api.github.com/repos/Ephoniz/Onky/releases?per_page=20";
#[cfg(windows)]
const MAX_DOWNLOAD_BYTES: u64 = 80 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Vec<Pre>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Pre {
    Num(u64),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateOffer {
    pub tag: String,
    pub setup_name: String,
    pub setup_url: String,
    pub checksum_url: String,
}

pub fn current_version() -> Option<Version> {
    parse_version(env!("CARGO_PKG_VERSION"))
}

pub fn parse_version(text: &str) -> Option<Version> {
    let text = text.trim().strip_prefix('v').unwrap_or(text.trim());
    let (core, pre) = text.split_once('-').unwrap_or((text, ""));
    let core = core.split_once('+').map(|(core, _)| core).unwrap_or(core);
    let mut parts = core.split('.');
    let major = parse_num(parts.next()?)?;
    let minor = parse_num(parts.next()?)?;
    let patch = parse_num(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    let mut pre_parts = Vec::new();
    if !pre.is_empty() {
        let pre = pre.split_once('+').map(|(pre, _)| pre).unwrap_or(pre);
        if pre.is_empty() {
            return None;
        }
        for part in pre.split('.') {
            if part.is_empty() {
                return None;
            }
            pre_parts.push(parse_pre(part)?);
        }
    }
    Some(Version {
        major,
        minor,
        patch,
        pre: pre_parts,
    })
}

pub fn is_newer(candidate: &Version, current: &Version) -> bool {
    compare(candidate, current) == Ordering::Greater
}

pub fn updates_enabled(settings: &str) -> bool {
    !settings
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("updates=off"))
}

pub fn updates_enabled_at(path: &Path) -> bool {
    match std::fs::read_to_string(path) {
        Ok(text) => updates_enabled(&text),
        Err(_) => true,
    }
}

pub fn settings_text(enabled: bool) -> &'static str {
    if enabled {
        "updates=on\n"
    } else {
        "updates=off\n"
    }
}

pub fn updates_setting_path() -> Option<PathBuf> {
    let root = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(root)
            .join("Audio Program")
            .join("settings.txt"),
    )
}

pub fn set_updates_enabled(enabled: bool) -> std::io::Result<()> {
    let path = updates_setting_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no settings directory")
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, settings_text(enabled))
}

pub fn checksum_matches(sums: &str, file_name: &str, bytes: &[u8]) -> bool {
    let Some(expected) = hash_for(sums, file_name) else {
        return false;
    };
    expected.eq_ignore_ascii_case(&sha256_hex(bytes))
}

#[derive(Debug)]
pub struct ReleaseListError;

pub fn newest_offer(
    current: &Version,
    releases_json: &str,
) -> Result<Option<UpdateOffer>, ReleaseListError> {
    let releases: Vec<Release> =
        serde_json::from_str(releases_json).map_err(|_| ReleaseListError)?;
    let mut best: Option<(Version, UpdateOffer)> = None;
    for release in releases {
        if release.draft {
            continue;
        }
        let Some(version) = parse_version(&release.tag_name) else {
            continue;
        };
        if !is_newer(&version, current) {
            continue;
        }
        let setup_name = format!(
            "AudioProgram-Setup-{}.{}.{}{}.exe",
            version.major,
            version.minor,
            version.patch,
            pre_suffix(&version)
        );
        let Some(setup) = release.assets.iter().find(|asset| asset.name == setup_name) else {
            continue;
        };
        let Some(sums) = release
            .assets
            .iter()
            .find(|asset| asset.name == "SHA256SUMS.txt")
        else {
            continue;
        };
        if !setup.browser_download_url.starts_with("https://")
            || !sums.browser_download_url.starts_with("https://")
        {
            continue;
        }
        let offer = UpdateOffer {
            tag: release.tag_name,
            setup_name,
            setup_url: setup.browser_download_url.clone(),
            checksum_url: sums.browser_download_url.clone(),
        };
        if best
            .as_ref()
            .is_none_or(|(best_version, _)| is_newer(&version, best_version))
        {
            best = Some((version, offer));
        }
    }
    Ok(best.map(|(_, offer)| offer))
}

fn pre_suffix(version: &Version) -> String {
    if version.pre.is_empty() {
        return String::new();
    }
    let mut suffix = String::from("-");
    for (index, part) in version.pre.iter().enumerate() {
        if index > 0 {
            suffix.push('.');
        }
        match part {
            Pre::Num(number) => suffix.push_str(&number.to_string()),
            Pre::Text(text) => suffix.push_str(text),
        }
    }
    suffix
}

fn parse_num(text: &str) -> Option<u64> {
    if text.is_empty() || (text.len() > 1 && text.starts_with('0')) {
        return None;
    }
    if !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn parse_pre(text: &str) -> Option<Pre> {
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Some(Pre::Num(parse_num(text)?));
    }
    if text
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Some(Pre::Text(text.to_string()));
    }
    None
}

fn compare(left: &Version, right: &Version) -> Ordering {
    (left.major, left.minor, left.patch)
        .cmp(&(right.major, right.minor, right.patch))
        .then_with(|| match (left.pre.is_empty(), right.pre.is_empty()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => compare_pre(&left.pre, &right.pre),
        })
}

fn compare_pre(left: &[Pre], right: &[Pre]) -> Ordering {
    for (left, right) in left.iter().zip(right.iter()) {
        let order = match (left, right) {
            (Pre::Num(left), Pre::Num(right)) => left.cmp(right),
            (Pre::Num(_), Pre::Text(_)) => Ordering::Less,
            (Pre::Text(_), Pre::Num(_)) => Ordering::Greater,
            (Pre::Text(left), Pre::Text(right)) => left.cmp(right),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}

fn hash_for(sums: &str, file_name: &str) -> Option<String> {
    for line in sums.lines() {
        let line = line.trim_end_matches('\r');
        let Some((hash, name)) = line.split_once("  ") else {
            continue;
        };
        if name == file_name
            && hash.len() == 64
            && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Some(hash.to_string());
        }
    }
    None
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        hex.push(char::from(b"0123456789abcdef"[usize::from(byte & 0xf)]));
    }
    hex
}

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    assets: Vec<Asset>,
}

#[derive(serde::Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[cfg(not(windows))]
pub fn apply_pending_update() {}

#[cfg(windows)]
pub fn apply_pending_update() {
    let Some(path) = updates_setting_path() else {
        return;
    };
    if !updates_enabled_at(&path) {
        return;
    }
    let Some(current) = current_version() else {
        return;
    };
    let Ok(body) = fetch_text(RELEASES_URL) else {
        return;
    };
    let Ok(Some(offer)) = newest_offer(&current, &body) else {
        return;
    };
    let Ok(setup) = fetch_bytes(&offer.setup_url) else {
        return;
    };
    let Ok(sums) = fetch_text(&offer.checksum_url) else {
        return;
    };
    if !checksum_matches(&sums, &offer.setup_name, &setup) {
        crate::startup::write_startup_log(
            "update checksum did not match; the installer was not run",
        );
        return;
    }
    let dir = std::env::temp_dir().join("AudioProgram-Update");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let setup_path = dir.join(&offer.setup_name);
    if std::fs::write(&setup_path, &setup).is_err() {
        return;
    }
    let _ = install_and_restart(&setup_path);
}

#[cfg(windows)]
fn fetch_text(url: &str) -> Result<String, ()> {
    let bytes = fetch_bytes(url)?;
    String::from_utf8(bytes).map_err(|_| ())
}

#[cfg(windows)]
fn fetch_bytes(url: &str) -> Result<Vec<u8>, ()> {
    if !url.starts_with("https://") {
        return Err(());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(8))
        .build();
    let response = agent
        .get(url)
        .set("User-Agent", "Audio-Program")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|_| ())?;
    use std::io::Read;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_DOWNLOAD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err(());
    }
    Ok(bytes)
}

#[cfg(windows)]
fn install_and_restart(setup: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    let app = std::env::current_exe()?;
    let command = format!(
        "Start-Process -LiteralPath '{}' -ArgumentList '/SILENT','/SUPPRESSMSGBOXES','/NORESTART' -Wait; Start-Process -LiteralPath '{}'",
        escape_powershell(setup),
        escape_powershell(&app)
    );
    // Detach so the installer can replace this exe, then start it again.
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &command])
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()?;
    std::process::exit(0);
}

#[cfg(windows)]
fn escape_powershell(path: &Path) -> String {
    path.display().to_string().replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::{
        checksum_matches, is_newer, newest_offer, parse_version, updates_enabled,
        updates_enabled_at,
    };
    use std::io::Write;

    fn version(text: &str) -> super::Version {
        parse_version(text).unwrap_or_else(|| panic!("bad version {text}"))
    }

    #[test]
    fn newer_versions_sort_by_semver() {
        assert!(is_newer(&version("0.1.1"), &version("0.1.0")));
        assert!(is_newer(&version("v0.2.0"), &version("0.1.9")));
        assert!(is_newer(&version("1.0.0"), &version("1.0.0-rc.1")));
        assert!(is_newer(
            &version("0.1.0-alpha.2"),
            &version("0.1.0-alpha.1")
        ));
        assert!(is_newer(&version("0.1.0-alpha.1"), &version("0.1.0-alpha")));
        assert!(is_newer(&version("0.1.0-beta"), &version("0.1.0-alpha")));
        assert!(is_newer(&version("0.1.0-rc.1"), &version("0.1.0-beta.9")));
        assert!(!is_newer(&version("0.1.0"), &version("0.1.0")));
        assert!(!is_newer(&version("0.1.0-alpha.1"), &version("0.1.0")));
        assert!(!is_newer(&version("0.1.0"), &version("0.1.1")));
        assert!(parse_version("1.2").is_none());
        assert!(parse_version("01.2.3").is_none());
    }

    #[test]
    fn checksum_accepts_only_the_matching_file() {
        let bytes = b"abc";
        let sums = concat!(
            "\n",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  AudioProgram-Setup-0.1.1.exe\n",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  other.exe\n",
        );
        assert!(checksum_matches(
            sums,
            "AudioProgram-Setup-0.1.1.exe",
            bytes
        ));
        assert!(!checksum_matches(
            sums,
            "AudioProgram-Setup-0.1.1.exe",
            b"nope"
        ));
        assert!(!checksum_matches(sums, "missing.exe", bytes));
        assert!(!checksum_matches(
            "not-a-checksum  AudioProgram-Setup-0.1.1.exe\n",
            "AudioProgram-Setup-0.1.1.exe",
            bytes
        ));
    }

    #[test]
    fn newest_release_skips_drafts_and_files_without_checksums() {
        let body = r#"[
          {"tag_name":"v0.2.0","draft":true,"assets":[
            {"name":"AudioProgram-Setup-0.2.0.exe","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.2.0/AudioProgram-Setup-0.2.0.exe"},
            {"name":"SHA256SUMS.txt","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.2.0/SHA256SUMS.txt"}
          ]},
          {"tag_name":"v0.1.2","draft":false,"assets":[
            {"name":"AudioProgram-Setup-0.1.2.exe","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.1.2/AudioProgram-Setup-0.1.2.exe"}
          ]},
          {"tag_name":"v0.1.1","draft":false,"assets":[
            {"name":"AudioProgram-Setup-0.1.1.exe","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.1.1/AudioProgram-Setup-0.1.1.exe"},
            {"name":"SHA256SUMS.txt","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.1.1/SHA256SUMS.txt"}
          ]},
          {"tag_name":"v0.1.0","draft":false,"assets":[]}
        ]"#;
        let offer = newest_offer(&version("0.1.0"), body)
            .unwrap()
            .expect("offer");
        assert_eq!(offer.tag, "v0.1.1");
        assert_eq!(offer.setup_name, "AudioProgram-Setup-0.1.1.exe");
        assert!(newest_offer(&version("0.1.1"), body).unwrap().is_none());
        assert!(newest_offer(&version("0.1.0"), "not json").is_err());
    }

    #[test]
    fn this_build_version_parses() {
        assert!(super::current_version().is_some());
    }

    #[test]
    fn first_alpha_sees_later_alphas_and_the_release_as_newer() {
        let alpha = version("0.1.0-alpha.1");
        for newer in ["0.1.0-alpha.2", "0.1.0-alpha.10", "0.1.0-beta.1", "0.1.0"] {
            assert!(is_newer(&version(newer), &alpha), "{newer}");
        }
        assert!(!is_newer(&version("0.1.0-alpha.1"), &alpha));
        assert!(is_newer(
            &version("0.1.0-alpha.10"),
            &version("0.1.0-alpha.2")
        ));
    }

    #[test]
    fn newest_prerelease_offer_uses_its_setup_name() {
        let body = r#"[
          {"tag_name":"v0.1.0-alpha.10","draft":false,"assets":[
            {"name":"AudioProgram-Setup-0.1.0-alpha.10.exe","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.1.0-alpha.10/AudioProgram-Setup-0.1.0-alpha.10.exe"},
            {"name":"SHA256SUMS.txt","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.1.0-alpha.10/SHA256SUMS.txt"}
          ]},
          {"tag_name":"v0.1.0-alpha.2","draft":false,"assets":[
            {"name":"AudioProgram-Setup-0.1.0-alpha.2.exe","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.1.0-alpha.2/AudioProgram-Setup-0.1.0-alpha.2.exe"},
            {"name":"SHA256SUMS.txt","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.1.0-alpha.2/SHA256SUMS.txt"}
          ]},
          {"tag_name":"v0.1.0-alpha.1","draft":false,"assets":[
            {"name":"AudioProgram-Setup-0.1.0-alpha.1.exe","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.1.0-alpha.1/AudioProgram-Setup-0.1.0-alpha.1.exe"},
            {"name":"SHA256SUMS.txt","browser_download_url":"https://github.com/Ephoniz/Onky/releases/download/v0.1.0-alpha.1/SHA256SUMS.txt"}
          ]}
        ]"#;
        let offer = newest_offer(&version("0.1.0-alpha.1"), body)
            .unwrap()
            .expect("offer");
        assert_eq!(offer.tag, "v0.1.0-alpha.10");
        assert_eq!(offer.setup_name, "AudioProgram-Setup-0.1.0-alpha.10.exe");
        assert!(newest_offer(&version("0.1.0-alpha.10"), body)
            .unwrap()
            .is_none());
    }

    #[test]
    fn updates_stay_on_unless_the_setting_says_off() {
        assert!(updates_enabled(""));
        assert!(updates_enabled("updates=on\n"));
        assert!(!updates_enabled("updates=off\n"));
        assert!(!updates_enabled("  updates=off  \n"));
        let dir =
            std::env::temp_dir().join(format!("audio-program-update-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.txt");
        assert!(updates_enabled_at(&path));
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "updates=off").unwrap();
        assert!(!updates_enabled_at(&path));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
