use std::error::Error;

use super::error::LexicalError;

/// 词法分析
pub fn lexical_analysis(input: String) {
    let mut after_preprocessing = preprocess(input);
}

const KEYWORDS: [&str; 32] = [
    "char",
    "double",
    "enum",
    "float", // 数据类型关键字
    "int",
    "long",
    "short",
    "signed",
    "struct",
    "union",
    "unsigned",
    "void",
    "for",
    "do",
    "while",
    "break",
    "continue", // 控制语句关键字
    "if",
    "else",
    "goto",
    "switch",
    "case",
    "functionault",
    "return",
    "auto",
    "extern",
    "register",
    "static", // 存储类型关键字
    "const",
    "sizeof",
    "typefunction",
    "volatile", // 其他关键字
];

const OPERATOR: [&str; 24] = [
    "+", "-", "*", "/", "%", "++", "--", // 算术运算符
    "==", "!=", ">", "<", ">=", "<=", // 关系运算符
    "&", "|", // 按位与，按位或（也是逻辑运算符的先导符）
    "&&", "||", "!", // 逻辑运算符
    "=", "+=", "-=", "+=", "/=", "%=", // 赋值运算符
];

/// 处理
fn process(input: String) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(vec![])
}

/// 预处理输入
///
/// 1. 去除注释
/// 2. 删除首尾空格，删除空行，按空格分割转为 Vec
fn preprocess(input: String) -> Result<Vec<String>, Box<dyn Error>> {
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
fn remove_comment(input: String) -> Result<String, Box<dyn Error>> {
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
                                return Err(Box::new(LexicalError::new(&format!(
                                    "multiline comment not closed at {}:{}",
                                    start_pos.0, start_pos.1
                                ))));
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
}
