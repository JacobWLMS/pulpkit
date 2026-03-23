//! Greetd IPC client — communicate with the greetd login manager.
//!
//! Greetd uses a simple JSON protocol over a Unix socket at $GREETD_SOCK.
//! This module provides a blocking client for creating/canceling sessions
//! and starting them with a given command.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

#[derive(Debug)]
pub enum GreetdError {
    NoSocket,
    Io(std::io::Error),
    Json(serde_json::Error),
    Protocol(String),
}

impl std::fmt::Display for GreetdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GreetdError::NoSocket => write!(f, "GREETD_SOCK not set"),
            GreetdError::Io(e) => write!(f, "IO: {e}"),
            GreetdError::Json(e) => write!(f, "JSON: {e}"),
            GreetdError::Protocol(msg) => write!(f, "greetd: {msg}"),
        }
    }
}

impl From<std::io::Error> for GreetdError {
    fn from(e: std::io::Error) -> Self { GreetdError::Io(e) }
}
impl From<serde_json::Error> for GreetdError {
    fn from(e: serde_json::Error) -> Self { GreetdError::Json(e) }
}

/// A greetd IPC connection.
pub struct GreetdClient {
    stream: UnixStream,
}

impl GreetdClient {
    /// Connect to the greetd socket.
    pub fn connect() -> Result<Self, GreetdError> {
        let sock = std::env::var("GREETD_SOCK").map_err(|_| GreetdError::NoSocket)?;
        let stream = UnixStream::connect(&sock)?;
        Ok(Self { stream })
    }

    /// Send a request and read the response.
    fn request(&mut self, req: serde_json::Value) -> Result<serde_json::Value, GreetdError> {
        let payload = serde_json::to_vec(&req)?;
        let len = payload.len() as u32;
        self.stream.write_all(&len.to_ne_bytes())?;
        self.stream.write_all(&payload)?;

        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf)?;
        let resp_len = u32::from_ne_bytes(len_buf) as usize;
        let mut resp_buf = vec![0u8; resp_len];
        self.stream.read_exact(&mut resp_buf)?;
        Ok(serde_json::from_slice(&resp_buf)?)
    }

    /// Create a session for the given username.
    pub fn create_session(&mut self, username: &str) -> Result<serde_json::Value, GreetdError> {
        self.request(serde_json::json!({
            "type": "create_session",
            "username": username
        }))
    }

    /// Post an authentication answer (e.g., password).
    pub fn post_auth(&mut self, response: Option<&str>) -> Result<serde_json::Value, GreetdError> {
        self.request(serde_json::json!({
            "type": "post_auth_message_response",
            "response": response
        }))
    }

    /// Start the session with the given command.
    pub fn start_session(&mut self, cmd: &[&str]) -> Result<serde_json::Value, GreetdError> {
        self.request(serde_json::json!({
            "type": "start_session",
            "cmd": cmd
        }))
    }

    /// Cancel the current session.
    pub fn cancel_session(&mut self) -> Result<serde_json::Value, GreetdError> {
        self.request(serde_json::json!({
            "type": "cancel_session"
        }))
    }
}
