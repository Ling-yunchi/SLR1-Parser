use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct LexicalError {
    pub message: String,
}

impl LexicalError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl Display for LexicalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for LexicalError {
    fn description(&self) -> &str {
        &self.message
    }
}
