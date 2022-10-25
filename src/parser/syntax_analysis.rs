use super::{
    error::{GrammarError, SyntaxError},
    lexical_analysis::Token,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    vec,
};

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
    /// 从yml中读取语法定义
    fn from_yml(input: &str) -> Result<Grammar, serde_yaml::Error> {
        serde_yaml::from_str::<Grammar>(input)
    }

    /// 验证语法定义是否合法
    fn validate(&self) -> Result<(), GrammarError> {
        // 验证终结符和非终结符没有重复元素
        let s = self.v.iter().chain(self.t.iter()).collect::<HashSet<_>>();
        if s.len() != self.v.len() + self.t.len() {
            return Err(GrammarError::new("终结符和非终结符存在重复元素"));
        }

        // 验证开始符号是否在非终结符集中
        if !self.v.contains(&self.s) {
            return Err(GrammarError::new("开始符号不在非终结符集中"));
        }

        // 验证产生式左部是否在非终结符集中
        for product in &self.p {
            if !self.v.contains(&product.left) {
                return Err(GrammarError::new("产生式左部不在非终结符集中"));
            }
        }

        // 验证产生式右部是否在非终结符集和终结符集中
        for product in &self.p {
            for right in &product.right {
                if !self.v.contains(right) && !self.t.contains(right) {
                    return Err(GrammarError::new("产生式右部不在非终结符集和终结符集中"));
                }
            }
        }

        Ok(())
    }
}

const GRAMMAR_YML: &str = "grammar.yml";

pub fn syntax_analysis(tokens: Vec<Token>) -> Result<(), SyntaxError> {
    let grammar_yml = std::fs::read_to_string(GRAMMAR_YML)
        .map_err(|e| SyntaxError::new(&format!("Failed to read grammar file error: {}", e)))?;
    let grammar = Grammar::from_yml(&grammar_yml)
        .map_err(|e| SyntaxError::new(&format!("Failed to parse grammar file error: {}", e)))?;
    grammar
        .validate()
        .map_err(|e| SyntaxError::new(&format!("Grammar valida error: {}", e)))?;

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
///
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

    x_first = x_first
        .into_iter()
        .chain(y_first.into_iter())
        .collect::<HashSet<String>>()
        .into_iter()
        .collect();
    let after = x_first.len();

    first.insert(x.to_string(), x_first);

    before < after
}

fn get_follow(g: &Grammar) -> HashMap<String, Vec<String>> {
    let mut first = get_first(g);
    get_follow_with_first(g, &mut first)
}

fn get_follow_with_first(
    g: &Grammar,
    first: &mut HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
    let mut follow = HashMap::new();

    // 初始化 follow 集合
    g.v.iter().for_each(|v| {
        follow.insert(v.clone(), vec![]);
    });

    // 将 #(句子结束符) 加入 S 的 follow 集合
    follow.get_mut(&g.s).unwrap().push("#".to_string());

    // 对于每个产生式 A->αBβ，将 FIRST(β) 去掉ε后加入 FOLLOW(B)
    let mut changed = true;
    while changed {
        changed = false;
        g.p.iter().for_each(|p| {
            for i in 0..p.right.len() {
                // 若产生式右部第一个符号为非终结符，即 X->Y...
                // 则继续向后遍历直到找到非终结符
                if !g.v.contains(&p.right[i]) {
                    continue;
                }

                // 找到第一个非终结符B
                // 若产生式为 A -> αB 型，将FOLLOW(A)加入FOLLOW(B)
                if i == p.right.len() - 1 {
                    changed = union_follow(&mut follow, &p.right[i], &p.left);
                    continue;
                }
                // 若产生式为 A -> αBβ 型，进行讨论
                else {
                    let beta = &p.right[i + 1..].to_vec();
                    let beta_first = get_first_all(first, beta);

                    // 若β的first集合中含有ε，则同 A -> αB 型，将FOLLOW(A)加入FOLLOW(B)
                    if beta_first.contains(&"ε".to_string()) {
                        changed = union_follow(&mut follow, &p.right[i], &p.left);
                    }

                    // 否则将FITST(β)去除ε后加入FOLLOW(B)
                    // if !follow.contains_key(&p.right[i]) {
                    //     follow.insert(p.right[i].clone(), vec![]);
                    // }

                    let mut b_follow = follow.get(&p.right[i]).unwrap().clone();
                    let before = b_follow.len();

                    b_follow = b_follow
                        .into_iter()
                        .chain(
                            beta_first
                                .into_iter()
                                .filter(|s| s != &&"ε".to_string())
                                .cloned(),
                        )
                        .collect::<HashSet<String>>()
                        .into_iter()
                        .collect();
                    let after = b_follow.len();

                    follow.insert(p.right[i].clone(), b_follow);

                    changed = changed || before < after;
                }
            }
        });
    }

    follow
}

/// 将 y follow 集合中的终结符添加到 x follow 集合中
///
/// @return x 的 follow 集是否发生了变化
fn union_follow(follow: &mut HashMap<String, Vec<String>>, x: &str, y: &str) -> bool {
    let mut x_first = follow.get(x).unwrap().clone();
    let before = x_first.len();

    let y_first = match y {
        "ε" => vec!["ε".to_string()],
        _ => follow.get(y).unwrap().clone(),
    };

    x_first = x_first
        .into_iter()
        .chain(y_first.into_iter())
        .collect::<HashSet<String>>()
        .into_iter()
        .collect();
    let after = x_first.len();

    follow.insert(x.to_string(), x_first);

    before < after
}

/// 求a的所有first集，a = Y_1...Y_n
/// 并将其加入到first集中
fn get_first_all<'a>(
    first: &'a mut HashMap<String, Vec<String>>,
    a: &Vec<String>,
) -> &'a Vec<String> {
    // 如果a是单个非终结符或者终结符，则FIRST(a)之前已经求过，直接返回即可
    if a.len() == 1 {
        // a = ε 特殊情况
        // if a.contains(&"ε".to_string()) {
        //     first.insert("ε".to_string(), vec!["ε".to_string()]);
        // }
        return first.get(&a[0]).unwrap();
    }

    let a_key = a.join(" ");
    if first.contains_key(&a_key) {
        return first.get(&a_key).unwrap();
    }

    first.insert(a_key.clone(), vec![]);

    let mut need_epsilon = true;
    for i in 0..a.len() {
        if first.get(&a[i]).unwrap().contains(&"ε".to_string()) {
            union_first(first, &a_key, &a[i], true);
        } else {
            need_epsilon = false;
            union_first(first, &a_key, &a[i], true);
            break;
        }
    }

    if need_epsilon {
        union_first(first, &a_key, "ε", false);
    }

    first.get(&a_key).unwrap()
}

#[cfg(test)]
mod tests {
    use super::Grammar;
    use crate::parser::syntax_analysis::{get_first, get_follow};

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

    const grammar_yml: &str = r#"
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

    #[test]
    fn test_first() {
        let g = Grammar::from_yml(grammar_yml).unwrap();
        println!("{:#?}", g);
        let mut first = get_first(&g);
        println!("{:#?}", first);
        first.iter_mut().for_each(|(k, v)| {
            v.sort();
        });

        assert_eq!(
            first,
            hashmap!(
                s!("E") => vec![s!("("),s!("id")],
                s!("E'") => vec![s!("+"),s!("ε")],
                s!("F") => vec![s!("("),s!("id")],
                s!("T") => vec![s!("("),s!("id")],
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

    #[test]
    fn test_follow() {
        let g = Grammar::from_yml(grammar_yml).unwrap();
        println!("{:#?}", g);
        let mut follow = get_follow(&g);
        println!("{:#?}", follow);
        follow.iter_mut().for_each(|(k, v)| {
            v.sort();
        });

        assert_eq!(
            follow,
            hashmap!(
                s!("E") => vec![s!("#"),s!(")")],
                s!("E'") => vec![s!("#"),s!(")")],
                s!("F") => vec![s!("#"),s!(")"),s!("*"),s!("+")],
                s!("T") => vec![s!("#"),s!(")"),s!("+")],
                s!("T'") => vec![s!("#"),s!(")"),s!("+")]
            )
        );
    }
}
