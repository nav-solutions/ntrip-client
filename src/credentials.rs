//! Client credentials
use base64::{engine::general_purpose, Engine};

#[cfg(doc)]
use crate::NTRIPClient;

/// [NTRIPCredentials] optionally used by [NTRIPClient]s
#[derive(Clone, Default, PartialEq)]
pub struct NTRIPCredentials {
    user: String,
    password: String,
}

impl NTRIPCredentials {
    pub fn new(user: &str, password: &str) -> Self {
        Self {
            user: user.to_string(),
            password: password.to_string(),
        }
    }

    pub fn encode(&self) -> String {
        general_purpose::STANDARD.encode(format!("{}:{}", self.user, self.password))
    }
}
