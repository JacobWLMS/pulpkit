//! PAM authentication — verify passwords for lock screen.
//!
//! Uses the `pam` system library via command execution (avoids native pam crate dep).
//! For a lock screen, we verify the current user's password by attempting a PAM auth.

use std::process::{Command, Stdio};
use std::io::Write;

/// Verify a password for the current user.
///
/// Uses `su` with the current username to check if the password is correct.
/// Returns `true` if authentication succeeds.
///
/// Note: This is a simple approach. A production implementation would use
/// the PAM C library directly via `pam-sys` or `pam` crate.
pub fn verify_password(password: &str) -> bool {
    let username = std::env::var("USER").unwrap_or_else(|_| "root".into());
    verify_password_for_user(&username, password)
}

/// Verify a password for a specific user.
pub fn verify_password_for_user(username: &str, password: &str) -> bool {
    // Use `su` to verify — it uses PAM internally
    let mut child = match Command::new("su")
        .arg("-c")
        .arg("true")
        .arg(username)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    if let Some(ref mut stdin) = child.stdin {
        let _ = writeln!(stdin, "{password}");
    }
    drop(child.stdin.take());

    child
        .wait()
        .map(|status| status.success())
        .unwrap_or(false)
}
