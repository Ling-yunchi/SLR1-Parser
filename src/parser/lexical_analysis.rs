use core::num;
use std::{error::Error, process::id};

use super::error::LexicalError;

/// 词法分析
pub fn lexical_analysis(input: String) -> Result<(Vec<Token>, bool), LexicalError> {
    let after_preprocessing = preprocess(input)?;
    Ok(process(after_preprocessing))
}

#[rustfmt::skip]
const KEYWORDS: [&str; 32] = [
    "char", "double", "enum", "float",  // 数据类型关键字
    "int", "long", "short", "signed",
    "struct", "union", "unsigned", "void",
    "for", "do", "while", "break", "continue",  // 控制语句关键字
    "if", "else", "goto",
    "switch", "case", "default", "return",
    "auto", "extern", "register", "static",  // 存储类型关键字
    "const", "sizeof", "typeof", "volatile"  // 其他关键字
];

#[rustfmt::skip]
const OPERATOR: [&str; 24] = [
    "+", "-", "*", "/", "%", "++", "--", // 算术运算符
    "==", "!=", ">", "<", ">=", "<=", // 关系运算符
    "&", "|", // 按位与，按位或（也是逻辑运算符的先导符）
    "&&", "||", "!", // 逻辑运算符
    "=", "+=", "-=", "+=", "/=", "%=", // 赋值运算符
];

#[rustfmt::skip]
const DELIMITERS: [&str; 9] = ["{", "}", "[", "]", "(", ")", ",", ".", ";"];

#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub token_value: String,
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.token_type == other.token_type && self.token_value == other.token_value
    }
}

#[derive(Debug)]
pub enum TokenType {
    Keyword,
    Identifier,
    Constant,
    Operator,
    Delimiter,
    Error(LexicalError),
}

impl PartialEq for TokenType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Error(l0), Self::Error(r0)) => l0.to_string() == r0.to_string(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

/// 处理
fn process(input: Vec<String>) -> (Vec<Token>, bool) {
    let mut result = Vec::new();
    let mut error = false;

    for line in input {
        let mut chars = line.chars().peekable();

        while let Some(char) = chars.next() {
            // 空格 跳过
            if char == ' ' {
                continue;
            }

            // 分隔符
            if DELIMITERS.contains(&char.to_string().as_str()) {
                result.push(Token {
                    token_type: TokenType::Delimiter,
                    token_value: char.to_string(),
                });
                continue;
            }

            // 运算符
            if OPERATOR.contains(&char.to_string().as_str()) {
                let mut operator = char.to_string();

                if let Some(next_char) = chars.peek() {
                    let double_operator = format!("{}{}", operator, next_char);
                    if OPERATOR.contains(&double_operator.as_str()) {
                        chars.next();
                        operator = double_operator;
                    }
                }

                result.push(Token {
                    token_type: TokenType::Operator,
                    token_value: operator,
                });
                continue;
            }

            // 数字
            if char.is_ascii_digit() {
                let mut number = char.to_string();

                while let Some(next_char) = chars.peek() {
                    if next_char.is_ascii_digit() || *next_char == '.' {
                        number.push(*next_char);
                        chars.next();
                    } else {
                        break;
                    }
                }

                // 防止出现数字开头的非法标识符
                if number.len() == line.len()
                    || OPERATOR.contains(&chars.peek().unwrap().to_string().as_str())
                    || DELIMITERS.contains(&chars.peek().unwrap().to_string().as_str())
                    || chars.peek().unwrap() == &' '
                {
                    if number.contains('.') {
                        let idx = number.find('.').unwrap();
                        if number[idx + 1..].contains('.') {
                            result.push(Token {
                                token_type: TokenType::Error(LexicalError::new(
                                    "Invalid float number",
                                )),
                                token_value: number,
                            });
                            error = true;
                        } else {
                            result.push(Token {
                                token_type: TokenType::Constant,
                                token_value: number,
                            });
                        }
                    }
                } else {
                    while let Some(next_char) = chars.peek() {
                        if next_char == &' '
                            || OPERATOR.contains(&next_char.to_string().as_str())
                            || DELIMITERS.contains(&next_char.to_string().as_str())
                        {
                            break;
                        } else {
                            number.push(*next_char);
                            chars.next();
                        }
                    }
                    result.push(Token {
                        token_type: TokenType::Error(LexicalError::new("Invalid identifier")),
                        token_value: number,
                    });
                    error = true;
                }
                continue;
            }

            // 字符常数
            if char == '\'' {
                let mut constant = char.to_string();

                while let Some(next_char) = chars.peek() {
                    if *next_char == '\'' {
                        constant.push(*next_char);
                        chars.next();
                        break;
                    } else {
                        constant.push(*next_char);
                        chars.next();
                    }
                }

                if constant.len() == 3 {
                    result.push(Token {
                        token_type: TokenType::Constant,
                        token_value: constant,
                    });
                } else {
                    result.push(Token {
                        token_type: TokenType::Error(LexicalError::new(
                            "Invalid character constant",
                        )),
                        token_value: constant,
                    });
                    error = true;
                }
                continue;
            }

            // 字符串常数
            if char == '"' {
                let mut constant = char.to_string();

                while let Some(next_char) = chars.peek() {
                    if *next_char == '"' {
                        constant.push(*next_char);
                        chars.next();
                        break;
                    } else {
                        constant.push(*next_char);
                        chars.next();
                    }
                }

                // 字符串首尾必须有双引号
                if constant.len() >= 2 && constant.starts_with('"') && constant.ends_with('"') {
                    result.push(Token {
                        token_type: TokenType::Constant,
                        token_value: constant,
                    });
                } else {
                    result.push(Token {
                        token_type: TokenType::Error(LexicalError::new("Invalid string constant")),
                        token_value: constant,
                    });
                    error = true;
                }
                continue;
            }

            // 标识符 & 关键字
            let mut identifier = char.to_string();
            while let Some(next_char) = chars.peek() {
                if next_char == &' '
                    || OPERATOR.contains(&next_char.to_string().as_str())
                    || DELIMITERS.contains(&next_char.to_string().as_str())
                {
                    break;
                } else {
                    identifier.push(*next_char);
                    chars.next();
                }
            }

            if KEYWORDS.contains(&identifier.as_str()) {
                result.push(Token {
                    token_type: TokenType::Keyword,
                    token_value: identifier,
                });
            } else {
                // 首字母应为字母或下划线
                let first_char = identifier.chars().next().unwrap();
                if first_char == '_' || first_char.is_alphabetic() {
                    result.push(Token {
                        token_type: TokenType::Identifier,
                        token_value: identifier,
                    })
                } else {
                    result.push(Token {
                        token_type: TokenType::Error(LexicalError::new("Invalid identifier")),
                        token_value: identifier,
                    });
                }
            }
        }
    }

    (result, error)
}

/// 预处理输入
///
/// 1. 去除注释
/// 2. 删除首尾空格，删除空行，按空格分割转为 Vec
fn preprocess(input: String) -> Result<Vec<String>, LexicalError> {
    let input_remove_comment = remove_comment(input)?;

    let lines = input_remove_comment
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(lines)
}

/// 删除注释
///
/// 删除单行注释，删除多行注释
///
/// 单行注释 format: // xxx
/// 多行注释 format: /* xxx */
fn remove_comment(input: String) -> Result<String, LexicalError> {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    let mut row = 1;
    let mut column = 0;
    while let Some(char) = chars.next() {
        if char == '\n' {
            row += 1;
            column = 0;
            result.push(char);
        } else {
            column += 1;
            match char {
                '/' => match chars.next() {
                    Some(next) => match next {
                        '/' => {
                            while let Some(char) = chars.next() {
                                if char == '\n' {
                                    row += 1;
                                    column = 0;
                                    result.push(char);
                                    break;
                                }
                            }
                        }
                        '*' => {
                            let mut error = true;
                            let start_pos = (row, column);
                            while let Some(char) = chars.next() {
                                match char {
                                    '\n' => {
                                        row += 1;
                                        column = 0;
                                    }
                                    '*' => {
                                        if let Some(next) = chars.next() {
                                            if next == '/' {
                                                error = false;
                                                break;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if error {
                                return Err(LexicalError::new(&format!(
                                    "multiline comment not closed at {}:{}",
                                    start_pos.0, start_pos.1
                                )));
                            }
                        }
                        _ => {
                            result.push(char);
                            result.push(next);
                        }
                    },
                    None => {
                        result.push(char);
                    }
                },
                _ => {
                    result.push(char);
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::result;

    use super::*;

    #[test]
    fn test_remove_commnet() {
        let input = String::from(
            r#"
// This is a comment
int main() {
/* 
    This is a multiline comment
    int a = 1;
*/
    int a = /* this is a inline multiline comment */ 1;
    printf("Hello, world!");
    return 0;
}
"#,
        );
        let output = remove_comment(input).unwrap();
        assert_eq!(
            output,
            r#"

int main() {

    int a =  1;
    printf("Hello, world!");
    return 0;
}
"#
        );
    }

    #[test]
    fn test_remove_commnet_error() {
        let input = String::from(
            r#"
// This is a comment
int main() {
    int a = /* this is a inline multiline comment */ 1;
    /* 
    This is a multiline comment
    but not closed

    printf("Hello, world!");
    return 0;
}
"#,
        );
        let result = remove_comment(input);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "multiline comment not closed at 5:5"
        );
    }

    #[test]
    fn test_preprocess() {
        let input = String::from(
            r#"
// This is a comment
int main() {
    /* 
    This is a multiline comment
    int a = 1;
    */
    int a = /* this is a inline multiline comment */ 1;
    printf("Hello, world!");
    return 0;
    }
"#,
        );
        let output = preprocess(input).unwrap();
        assert_eq!(
            output,
            vec![
                "int main() {",
                "int a =  1;",
                "printf(\"Hello, world!\");",
                "return 0;",
                "}"
            ]
        );
    }

    #[test]
    fn test_lexical_analysis() {
        let code = String::from(
            r#"// This is a note.
            int main(int a, int b){
                int res;
                res = a + b;
                int d = a;
                /*
                if (a < 0){
                    res = -a;
                }
                else{
                    while (b > 0){
                        b = b-1;
                    }
                    res = 0;
                }
                */
            }"#,
        );
        let (result, error) = lexical_analysis(code).unwrap();
        #[rustfmt::skip]
        assert_eq!(
            result,
            vec![
                Token {token_type: TokenType::Keyword, token_value: "int".to_string()},
                Token {token_type: TokenType::Identifier, token_value: "main".to_string()},
                Token {token_type: TokenType::Delimiter, token_value: "(".to_string()},
                Token {token_type: TokenType::Keyword, token_value: "int".to_string()},
                Token {token_type: TokenType::Identifier, token_value: "a".to_string()},
                Token {token_type: TokenType::Delimiter, token_value: ",".to_string()},
                Token {token_type: TokenType::Keyword, token_value: "int".to_string()},
                Token {token_type: TokenType::Identifier, token_value: "b".to_string()},
                Token {token_type: TokenType::Delimiter, token_value: ")".to_string()},
                Token {token_type: TokenType::Delimiter, token_value: "{".to_string()},
                Token {token_type: TokenType::Keyword, token_value: "int".to_string()},
                Token {token_type: TokenType::Identifier, token_value: "res".to_string()},
                Token {token_type: TokenType::Delimiter, token_value: ";".to_string()},
                Token {token_type: TokenType::Identifier, token_value: "res".to_string()},
                Token {token_type: TokenType::Operator, token_value: "=".to_string()},
                Token {token_type: TokenType::Identifier, token_value: "a".to_string()},
                Token {token_type: TokenType::Operator, token_value: "+".to_string()},
                Token {token_type: TokenType::Identifier, token_value: "b".to_string()},
                Token {token_type: TokenType::Delimiter, token_value: ";".to_string()},
                Token {token_type: TokenType::Keyword, token_value: "int".to_string()},
                Token {token_type: TokenType::Identifier, token_value: "d".to_string()},
                Token {token_type: TokenType::Operator, token_value: "=".to_string()},
                Token {token_type: TokenType::Identifier, token_value: "a".to_string()},
                Token {token_type: TokenType::Delimiter, token_value: ";".to_string()},
                Token {token_type: TokenType::Delimiter, token_value: "}".to_string()}
            ]
        );
        assert!(!error);
    }

    #[test]
    fn test() {
        let identifier = String::from("abc");
        println!("{}", &identifier[0..1]);
        assert_eq!(&identifier[0..1], "a");
    }
}
