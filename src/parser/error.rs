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

#[derive(Debug)]
pub struct SyntaxError {
    pub message: String,
}

impl SyntaxError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for SyntaxError {
    fn description(&self) -> &str {
        &self.message
    }
}

#[derive(Debug)]
pub struct GrammarError {
    pub message: String,
}

impl GrammarError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl Display for GrammarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for GrammarError {
    fn description(&self) -> &str {
        &self.message
    }
}
