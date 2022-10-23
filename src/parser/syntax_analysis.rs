use std::{
    collections::{binary_heap::Iter, HashMap, HashSet},
    os::windows::prelude::FromRawSocket,
};

use serde::{Deserialize, Serialize};

use super::{error::SyntaxError, lexical_analysis::Token};

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub left: String,       // 产生式左部，为一个非终结符
    pub right: Vec<String>, // 产生式右部，含多个终结符或非终结符
}

/// 语法定义
#[derive(Debug, Serialize, Deserialize)]
pub struct Grammar {
    /// 开始符号
    pub s: String,
    /// 非终结符集
    pub v: Vec<String>,
    /// 终结符集
    pub t: Vec<String>,
    /// 产生式集
    pub p: Vec<Product>,
}

impl Grammar {
    fn from_yml(input: &str) -> Result<Grammar, serde_yaml::Error> {
        serde_yaml::from_str::<Grammar>(input)
    }
}

const GRAMMAR_YML: &str = "grammar.yml";

pub fn syntax_analysis(tokens: Vec<Token>) -> Result<(), SyntaxError> {
    let grammar_yml = std::fs::read_to_string(GRAMMAR_YML)
        .map_err(|e| SyntaxError::new(&format!("Failed to read grammar file error: {}", e)))?;
    let grammar = Grammar::from_yml(&grammar_yml)
        .map_err(|e| SyntaxError::new(&format!("Failed to parse grammar file error: {}", e)))?;

    // println!("{:?}", grammar);

    Ok(())
}

fn get_SLR1_table() -> Result<(), SyntaxError> {
    Ok(())
}

fn get_first(g: &Grammar) -> HashMap<String, Vec<String>> {
    let mut first = HashMap::new();
    // 终结符的 first 集合为自身
    g.t.iter().for_each(|t| {
        first.insert(t.clone(), vec![t.clone()]);
    });

    // 初始化非终结符的 first 集合为空
    g.v.iter().for_each(|v| {
        first.insert(v.clone(), vec![]);
    });

    // 若产生式右部第一个符号为终结符或右部含有ε，则将其加入该非终结符的 first 集合
    g.p.iter().for_each(|p| {
        if g.t.contains(&p.right[0]) {
            first.get_mut(&p.left).unwrap().push(p.right[0].clone());
        }
        // if p.right.contains(&"ε".to_string())
        //     && !first.get_mut(&p.left).unwrap().contains(&"ε".to_string())
        // {
        //     first.get_mut(&p.left).unwrap().push("ε".to_string());
        // }
    });

    // 对V中所有非终结符 X，检查产生式右部，添加 First(X) 中的终结符
    let mut changed = true;
    while changed {
        changed = false;
        g.p.iter().for_each(|p| {
            // 若产生式右部第一个符号为非终结符，即 X->Y...
            // 则将其加入该非终结符的 first 集合
            if g.v.contains(&p.right[0]) {
                changed = union_first(&mut first, &p.left, &p.right[0], true);
            }

            // 若产生式右部长度为一，即 X->Y，则不需要检查后续符号
            if p.right.len() == 1 {
                return;
            }

            // 产生式右部从第一个字符开始是连续的符号，即X->Y_1...Y_i...Y_k
            // 注: 此处Y_i若为终结符，则其 first 集合中不会包含 ε
            //     所以通过first集判断 Y_i->ε 不需要判断是否为终结符。

            // 是否从Y_1到Y_k的first集合中均包含ε，即 Y_1...Y_k->ε
            let mut need_epsilon = true;

            // 如果对于任何j，1<=j<=i-1，FIRST(Yj)都含有ε，
            // 则把FIRST(Yi)中所有非ε元素添加到FIRST(X)中
            for i in 0..p.right.len() {
                if first.get(&p.right[i]).unwrap().contains(&"ε".to_string()) {
                    changed = union_first(&mut first, &p.left, &p.right[i + 1], need_epsilon);
                } else {
                    need_epsilon = false;
                    break;
                }
            }

            // 若 Y_1...Y_k->ε，则将ε加入X的first集合
            if need_epsilon {
                changed = union_first(&mut first, &p.left, &"ε".to_string(), false);
            }
        });
    }

    first
}

/// 将 y first 集合中的终结符添加到 x first 集合中
/// @param discard 是否丢弃 y first 集合中的 ε
/// @return 是否发生了变化
fn union_first(first: &mut HashMap<String, Vec<String>>, x: &str, y: &str, discard: bool) -> bool {
    let mut x_first = first.get(x).unwrap().clone();
    let before = x_first.len();

    let y_first = match (y, discard) {
        ("ε", false) => vec!["ε".to_string()],
        ("ε", true) => vec![],
        (_, true) => first
            .get(y)
            .unwrap()
            .clone()
            .into_iter()
            .filter(|s| s != "ε")
            .collect(),
        (_, false) => first.get(y).unwrap().clone(),
    };

    println!("{:#?}", x_first);

    x_first = x_first
        .into_iter()
        .chain(y_first.into_iter())
        .collect::<HashSet<String>>()
        .into_iter()
        .collect();
    let after = x_first.len();

    println!("{:#?}", x_first);

    first.insert(x.to_string(), x_first);

    before < after
}

fn get_follow() -> Result<(), SyntaxError> {
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use crate::parser::syntax_analysis::get_first;

    use super::Grammar;

    #[test]
    fn test_serde_yml_read() {
        let yml = std::fs::read_to_string("grammar.yml").unwrap();
        let g: Grammar = serde_yaml::from_str(&yml).unwrap();
        println!("{:?}", g);
    }

    macro_rules! hashmap {
        ($( $key: expr => $val: expr ),*) => {{
             let mut map = ::std::collections::HashMap::new();
             $( map.insert($key, $val); )*
             map
        }}
    }

    macro_rules! s {
        ($s:expr) => {
            String::from($s)
        };
    }

    #[test]
    fn test_first() {
        let grammar_yml = r#"
        s: E
        v:
          - E
          - E'
          - T
          - T'
          - F
        t:
          - ε
          - +
          - "*"
          - (
          - )
          - id
        p:
          - left: E
            right:
              - T
              - E'
          - left: E'
            right:
              - +
              - T
              - E'
          - left: E'
            right:
              - ε
          - left: T
            right:
              - F
              - T'
          - left: T'
            right:
              - "*"
              - F
              - T'
          - left: T'
            right:
              - ε
          - left: F
            right:
              - (
              - E
              - )
          - left: F
            right:
              - id
        "#;
        let g = Grammar::from_yml(grammar_yml).unwrap();
        println!("{:#?}", g);
        let first = get_first(&g);
        println!("{:#?}", first);

        assert_eq!(
            first,
            hashmap!(
                s!("E") => vec![s!("id"),s!("(")],
                s!("E'") => vec![s!("+"),s!("ε")],
                s!("F") => vec![s!("("),s!("id")],
                s!("T") => vec![s!("id"),s!("(")],
                s!("T'") => vec![s!("*"),s!("ε")],
                s!("*") => vec![s!("*")],
                s!("+") => vec![s!("+")],
                s!("ε") => vec![s!("ε")],
                s!("(") => vec![s!("(")],
                s!(")") => vec![s!(")")],
                s!("id") => vec![s!("id")]
            )
        );
    }
}
