//! Manual checks against the project's public GitHub releases. Never executes downloads.
use meridian_protocol::{ErrorBody, ErrorCode, UpdateInfo};
use std::{process::Command, sync::Mutex};
static CHECKING: Mutex<()> = Mutex::new(());
fn error(message: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, message.to_string())
}
fn interpret(status: &str, body: &str, installed: &str) -> Result<UpdateInfo, ErrorBody> {
    let mut result = UpdateInfo {
        installed: installed.into(),
        latest: None,
        available: false,
        message: String::new(),
        published: None,
    };
    match status {
        "404" => {
            result.message = "No public release found. The repository may have no releases or may be private.".into();
            return Ok(result);
        }
        "403" | "429" => return Err(error("GitHub rate limit or access restriction. Try again later.")),
        "200" => (),
        _ => return Err(error(format!("GitHub returned HTTP {status}. Try again later."))),
    }
    let release: serde_json::Value =
        serde_json::from_str(body).map_err(|_| error("GitHub returned an invalid release response."))?;
    if release.get("draft").and_then(|v| v.as_bool()) != Some(false)
        || release.get("prerelease").and_then(|v| v.as_bool()) != Some(false)
    {
        return Err(error("GitHub did not return a stable published release."));
    }
    let tag = release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| error("GitHub release has no version tag."))?;
    result.latest = Some(tag.into());
    result.published = release.get("published_at").and_then(|v| v.as_str()).map(str::to_owned);
    match (semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag)), semver::Version::parse(installed)) {
        (Ok(latest), Ok(current)) => {
            result.available = latest > current;
            result.message = if result.available {
                "A newer Meridian release is available."
            } else if latest == current {
                "You have the latest published version."
            } else {
                "Your build is newer than the latest published release."
            }
            .into();
        }
        _ => {
            result.message =
                "A release was found, but its version tag cannot be compared automatically. Review it on GitHub.".into()
        }
    }
    Ok(result)
}
pub fn check() -> Result<UpdateInfo, ErrorBody> {
    let _guard = CHECKING.try_lock().map_err(|_| error("An update check is already running."))?;
    let output = Command::new("/usr/bin/curl")
        .args([
            "--disable",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "10",
            "--max-time",
            "30",
            "--max-filesize",
            "1048576",
            "--header",
            "Accept: application/vnd.github+json",
            "--header",
            "X-GitHub-Api-Version: 2022-11-28",
            "--user-agent",
            "Meridian-Update-Check",
            "--write-out",
            "\n%{http_code}",
            "https://api.github.com/repos/saikrishna488/meridian/releases/latest",
        ])
        .output()
        .map_err(|e| error(format!("Cannot check updates. Install curl and check your network: {e}")))?;
    if !output.status.success() {
        return Err(error("Could not reach GitHub. Check your network connection and try again."));
    }
    let reply = String::from_utf8(output.stdout).map_err(error)?;
    let (body, status) = reply.rsplit_once('\n').ok_or_else(|| error("Invalid GitHub response."))?;
    interpret(status.trim(), body, env!("CARGO_PKG_VERSION"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares_versions_numerically_and_handles_missing_or_unusual_releases() {
        let release = |tag: &str| format!(r#"{{"tag_name":"{tag}","draft":false,"prerelease":false}}"#);
        assert!(interpret("200", &release("v0.10.0"), "0.9.0").unwrap().available);
        assert!(!interpret("200", &release("v0.9.0"), "0.10.0").unwrap().available);
        assert!(!interpret("200", &release("v0.1.0"), "0.1.0").unwrap().available);
        assert!(interpret("200", &release("nightly"), "0.1.0").unwrap().message.contains("cannot be compared"));
        assert!(interpret("404", "{}", "0.1.0").unwrap().latest.is_none());
        assert!(interpret("403", "{}", "0.1.0").is_err());
        assert!(interpret("200", "{}", "0.1.0").is_err());
        assert!(interpret("200", "not json", "0.1.0").is_err());
    }
}
