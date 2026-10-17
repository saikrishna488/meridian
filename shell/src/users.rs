//! Local account management through AccountsService's authenticated D-Bus API.
use glib::variant::ToVariant;
use gtk::{gio, glib};
use meridian_protocol::{ErrorBody, ErrorCode, UserAction, UserCreate, UserInfo};
use std::{
    io::Write,
    process::{Command, Stdio},
};
static MUTATING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
struct Mutation;
impl Mutation {
    fn start() -> Result<Self, ErrorBody> {
        MUTATING
            .compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst)
            .map_err(|_| error("Another account change is in progress."))?;
        Ok(Self)
    }
}
impl Drop for Mutation {
    fn drop(&mut self) {
        MUTATING.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
const ROOT: &str = "/org/freedesktop/Accounts";
const IFACE: &str = "org.freedesktop.Accounts";
fn error(e: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::new(ErrorCode::Failed, e.to_string())
}
async fn call(
    path: &str,
    interface: &str,
    method: &str,
    params: Option<&glib::Variant>,
) -> Result<glib::Variant, ErrorBody> {
    let bus = gio::bus_get_future(gio::BusType::System).await.map_err(error)?;
    bus.call_future(
        Some(IFACE),
        path,
        interface,
        method,
        params,
        None,
        gio::DBusCallFlags::ALLOW_INTERACTIVE_AUTHORIZATION,
        120_000,
    )
    .await
    .map_err(|e| error(format!("AccountsService: {e}")))
}
fn prop<T: glib::variant::FromVariant>(dict: &glib::Variant, key: &str) -> Option<T> {
    glib::VariantDict::new(Some(dict)).lookup::<T>(key).ok().flatten()
}
async fn entries() -> Result<Vec<(String, UserInfo)>, ErrorBody> {
    let result = call(ROOT, IFACE, "ListCachedUsers", None).await?;
    let paths =
        result.child_value(0).get::<Vec<glib::variant::ObjectPath>>().ok_or_else(|| error("Invalid account list"))?;
    let current = std::fs::read_to_string("/proc/self/status").ok().and_then(|s| {
        s.lines()
            .find(|l| l.starts_with("Uid:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u32>().ok())
    });
    let mut users = vec![];
    for path in paths {
        let path = path.as_str();
        let reply = call(
            path,
            "org.freedesktop.DBus.Properties",
            "GetAll",
            Some(&("org.freedesktop.Accounts.User",).to_variant()),
        )
        .await?;
        let dict = reply.child_value(0);
        if prop::<bool>(&dict, "SystemAccount").unwrap_or(true) {
            continue;
        }
        let uid = prop::<u64>(&dict, "Uid")
            .and_then(|id| u32::try_from(id).ok())
            .ok_or_else(|| error("Invalid account UID"))?;
        if uid == 0 {
            continue;
        }
        users.push((
            path.into(),
            UserInfo {
                uid,
                name: prop(&dict, "UserName").unwrap_or_default(),
                real_name: prop(&dict, "RealName").unwrap_or_default(),
                administrator: prop::<i32>(&dict, "AccountType") == Some(1),
                locked: prop(&dict, "Locked").unwrap_or(true),
                current: current == Some(uid),
                home: prop(&dict, "HomeDirectory").unwrap_or_default(),
            },
        ));
    }
    users.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    Ok(users)
}
pub async fn list() -> Result<Vec<UserInfo>, ErrorBody> {
    Ok(entries().await?.into_iter().map(|(_, u)| u).collect())
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name.bytes().next().is_some_and(|c| c.is_ascii_lowercase() || c == b'_')
        && name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_-".contains(&c))
}
fn hash_password(password: String) -> Result<String, ErrorBody> {
    if password.len() < 8 || password.len() > 1024 || password.contains(['\0', '\n', '\r']) {
        return Err(error("Use a password of 8–1024 bytes without line breaks."));
    }
    // Passwords never enter process arguments, logs, or shell commands.
    let mut child = Command::new("/usr/bin/openssl")
        .args(["passwd", "-6", "-stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(error)?;
    let write = child.stdin.take().ok_or_else(|| error("Password input unavailable"))?.write_all(password.as_bytes());
    let output = child.wait_with_output().map_err(error)?;
    write.map_err(error)?;
    let hash = String::from_utf8(output.stdout).map_err(error)?.trim().to_owned();
    if !output.status.success() || !hash.starts_with("$6$") {
        return Err(error("Password hashing failed. Check that OpenSSL is installed."));
    }
    Ok(hash)
}
pub async fn create(choice: UserCreate) -> Result<(), ErrorBody> {
    let _mutation = Mutation::start()?;
    if !valid_name(&choice.name) || choice.real_name.len() > 256 || choice.real_name.chars().any(char::is_control) {
        return Err(error(
            "Use a lowercase username with letters, numbers, underscores or hyphens (maximum 32 characters).",
        ));
    }
    let hash = crate::files::blocking(move || hash_password(choice.password)).await?;
    let reply = call(
        ROOT,
        IFACE,
        "CreateUser",
        Some(&(choice.name.as_str(), choice.real_name.as_str(), i32::from(choice.administrator)).to_variant()),
    )
    .await?;
    let path =
        reply.child_value(0).get::<glib::variant::ObjectPath>().ok_or_else(|| error("Invalid new account path"))?;
    call(path.as_str(), "org.freedesktop.Accounts.User", "SetPassword", Some(&(hash.as_str(), "").to_variant()))
        .await
        .map_err(|e| {
            error(format!("Account created, but password setup failed. Reset its password before use: {}", e.message))
        })?;
    Ok(())
}
fn protect_account(user: &UserInfo, action: &str, administrators: usize) -> Result<(), ErrorBody> {
    if matches!(action, "delete" | "lock" | "standard") {
        if user.current {
            return Err(error("You cannot delete, lock or demote your current account."));
        }
        if user.administrator && !user.locked && administrators <= 1 {
            return Err(error("Keep at least one unlocked administrator account."));
        }
    }
    Ok(())
}
pub async fn change(choice: UserAction) -> Result<(), ErrorBody> {
    let _mutation = Mutation::start()?;
    let entries = entries().await?;
    let (path, user) = entries
        .iter()
        .find(|(_, u)| u.uid == choice.uid)
        .ok_or_else(|| error("Account is no longer available. Refresh the list."))?;
    protect_account(user, &choice.action, entries.iter().filter(|(_, u)| u.administrator && !u.locked).count())?;
    let interface = "org.freedesktop.Accounts.User";
    match choice.action.as_str() {
        "delete" => {
            call(ROOT, IFACE, "DeleteUser", Some(&(i64::from(user.uid), false).to_variant())).await?;
        }
        "lock" | "unlock" => {
            call(path, interface, "SetLocked", Some(&(choice.action == "lock",).to_variant())).await?;
        }
        "administrator" | "standard" => {
            call(path, interface, "SetAccountType", Some(&(i32::from(choice.action == "administrator"),).to_variant()))
                .await?;
        }
        "password" => {
            let hash = crate::files::blocking(move || hash_password(choice.password.unwrap_or_default())).await?;
            call(path, interface, "SetPassword", Some(&(hash.as_str(), "").to_variant())).await?;
        }
        _ => return Err(error("Unknown account action.")),
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protects_current_account_and_last_unlocked_administrator() {
        let mut user = UserInfo {
            uid: 1000,
            name: "owner".into(),
            real_name: String::new(),
            administrator: true,
            locked: false,
            current: true,
            home: String::new(),
        };
        for action in ["delete", "lock", "standard"] {
            assert!(protect_account(&user, action, 2).is_err());
        }
        assert!(protect_account(&user, "password", 1).is_ok());
        user.current = false;
        for action in ["delete", "lock", "standard"] {
            assert!(protect_account(&user, action, 1).is_err());
        }
        assert!(protect_account(&user, "delete", 2).is_ok());
        user.administrator = false;
        assert!(protect_account(&user, "delete", 1).is_ok());
    }
    #[test]
    fn account_names_and_passwords_are_validated() {
        for name in ["-root", "UPPER", "../user", "a b", "", "user\n"] {
            assert!(!valid_name(name));
        }
        assert!(valid_name("new_user-2"));
        assert!(hash_password("short".into()).is_err());
        assert!(hash_password("line\nbreak".into()).is_err());
    }
}
