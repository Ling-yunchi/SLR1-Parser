use serde::{ser::Error, Deserialize, Serialize};

use super::{error::SyntaxError, lexical_analysis::Token};

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub left: String,       // 产生式左部，为一个非终结符
    pub right: Vec<String>, // 产生式右部，含多个终结符或非终结符
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Grammar {
    pub s: String,       // 开始符号
    pub v: Vec<String>,  // 非终结符集
    pub t: Vec<String>,  // 终结符集
    pub p: Vec<Product>, // 产生式集
}

impl Grammar {
    fn from_yml(input: String) -> Result<Grammar, serde_yaml::Error> {
        serde_yaml::from_str::<Grammar>(&input)
    }
}

const GRAMMAR_YML: &str = "grammar.yml";

pub fn syntax_analysis(tokens: Vec<Token>) -> Result<(), SyntaxError> {
    let grammar_yml = std::fs::read_to_string(GRAMMAR_YML)
        .map_err(|e| SyntaxError::new(&format!("Failed to read grammar file error: {}", e)))?;
    let grammar = Grammar::from_yml(grammar_yml)
        .map_err(|e| SyntaxError::new(&format!("Failed to parse grammar file error: {}", e)))?;
    println!("{:?}", grammar);
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::Grammar;

    #[test]
    fn test_serde_yml_read() {
        let yml = std::fs::read_to_string("grammar.yml").unwrap();
        let g: Grammar = serde_yaml::from_str(&yml).unwrap();
        println!("{:?}", g);
    }
}
