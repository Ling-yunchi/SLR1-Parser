use super::{
    error::{GrammarError, SyntaxError},
    lexical_analysis::{Token, TokenType},
};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Display,
    vec,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub left: String,       // 产生式左部，为一个非终结符
    pub right: Vec<String>, // 产生式右部，含多个终结符或非终结符
}

impl Display for Product {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> ", self.left)?;
        for i in 0..self.right.len() {
            write!(f, "{} ", self.right[i])?;
        }
        Ok(())
    }
}

/// 语法定义
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn from_yml(input: &str) -> Result<Grammar, serde_yaml::Error> {
        serde_yaml::from_str::<Grammar>(input)
    }

    /// 验证语法定义是否合法
    pub fn validate(&self) -> Result<(), GrammarError> {
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
        .map_err(|e| SyntaxError::new(&format!("Grammar validate error: {}", e)))?;

    Ok(())
}

/// # 对输入文法G获取SLR(1)分析表
///
/// 获取ACTION表与GOTO表
/// 1. 将非拓广文法G转换为拓广文法G'
/// 2. 求解拓广文法G'的FOLLOW集，规约时使用
/// 3. 求解拓广文法G'的LR(0)项目集族
/// 4. 遍历项目集族，构造ACTION表与GOTO表
pub fn get_slr1_table(
    g: &Grammar,
) -> Result<(Vec<HashMap<String, String>>, Vec<HashMap<String, String>>), SyntaxError> {
    let mut outreach_g = g.clone();
    // 获取非拓广文法G的FOLLOW集，进行规约时使用
    let follow = get_follow(&outreach_g);

    // 将非拓广文法G转换为拓广文法G'
    // 即修改开始符号为S'，添加产生式S' -> S，并将S'加入非终结符集
    let raw_s = outreach_g.s.clone();
    outreach_g.s = raw_s.clone() + "'";
    outreach_g.v.push(outreach_g.s.clone());
    outreach_g.p.push(Product {
        left: outreach_g.s.clone(),
        right: vec![raw_s],
    });
    // 拓广文法的目的是保证文法的开始符号的定义只有一个产生式
    // 并且文法的开始符号不会出现在其他产生式的右部
    // 也保证了G'只有唯一的接受状态

    // 求解G'的LR(0)项目集族
    let lr0_items = get_lr0_collection(&outreach_g);

    // Action表初始化
    let mut ACTION = Vec::new();
    let mut row = HashMap::new();
    outreach_g.t.iter().for_each(|t| {
        row.insert(t.clone(), "".to_string());
    });
    row.insert("#".to_string(), "".to_string());
    lr0_items.iter().for_each(|_| {
        ACTION.push(row.clone());
    });

    // Goto表初始化
    let mut GOTO = Vec::new();
    let mut temp_v = outreach_g.v.clone().into_iter().collect::<HashSet<_>>();
    temp_v.remove((outreach_g.s.clone()).as_str());
    let mut row = HashMap::new();
    temp_v.iter().for_each(|v| {
        row.insert(v.clone(), "".to_string());
    });
    lr0_items.iter().for_each(|_| {
        GOTO.push(row.clone());
    });

    // 遍历LR(0)项目集族，填充Action表和Goto表
    // 1. 若项目A->α.aβ属于I_k，且GO(I_k,a)=I_j，a为终结符，则置ACTION[k,a]为sj
    // 2. 若项目A->α.属于I_k，那么对任何终结符a∈FOLLOW(A),置ACTION[k,a]为rj，假定A->α为G'的第j个产生式
    // 3. 若项目S'->S.属于I_k，则置ACTION[k,#]为“acc”
    // 4. 若GO(I_k,A)=I_j，A为非终结符，则置GOTO[k,A]=j
    // 5. 若不为以上情况，则ACTION与GOTO表剩余单元格置为空，代表出现错误
    for (i, items) in lr0_items.iter().enumerate() {
        for item in items.iter() {
            // 圆点不在LR(0)项目的最后，则需要移进
            if item.dot < item.right.len() {
                // 获取圆点项目的下一个字符
                let ch = &item.right[item.dot];
                // 找出项目集items在读入下一个字符ch后，转移到的项目集
                // 即找到使得goto(I, ch) = lr0_items[j]成立的j
                for (j, items1) in lr0_items.iter().enumerate() {
                    if items_eq(&goto(items, ch, &outreach_g), items1) {
                        // 如果ch为终结符，则将ACTION[i, ch]置为sj
                        if outreach_g.t.contains(ch) {
                            let action = format!("s{}", j);
                            match ACTION[i].insert(ch.clone(), action.clone()) {
                                Some(a) if a != "" && a != action => {
                                    warn!(
                                        "SLR action conflict: ACTION[{},\"{}\"] = {} or {}, use {}",
                                        i, ch, a, action, action
                                    );
                                }
                                _ => {}
                            }
                        }
                        // 如果ch为非终结符，则将GOTO[i, ch]置为j
                        else {
                            let goto = format!("{}", j);
                            match GOTO[i].insert(ch.clone(), goto.clone()) {
                                Some(g) if g != "" && g != goto => {
                                    warn!(
                                        "SLR goto conflict: GOTO[{},\"{}\"] = {} or {}, use {}",
                                        i, ch, g, goto, goto
                                    );
                                }
                                _ => {}
                            }
                        }
                        break;
                    }
                }
            }
            // 圆点在LR(0)项目的最后，则需要规约
            else {
                // 如果是S'->S.，则将ACTION[k, #]置为acc
                if item.left == outreach_g.s {
                    ACTION[i].insert("#".to_string(), "acc".to_string());
                }
                // 否则，对于任何终结符a∈FOLLOW(A)，将ACTION[k, a]置为rj
                else {
                    let j = outreach_g
                        .p
                        .iter()
                        .position(|p| p.left == item.left && p.right == item.right)
                        .unwrap();
                    let follow_left = follow.get(&item.left).unwrap();
                    for f in follow_left {
                        if outreach_g.t.contains(f) {
                            let action = format!("r{}", j);
                            match ACTION[i].insert(f.clone(), action.clone()) {
                                Some(a) if a != "" && a != action => {
                                    warn!(
                                        "SLR action conflict: ACTION[{},\"{}\"] = {} or {}, use {}",
                                        i, f, a, action, action
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    if follow_left.contains(&"#".to_string()) {
                        ACTION[i].insert("#".to_string(), format!("r{}", j));
                    }
                }
            }
        }
    }

    Ok((ACTION, GOTO))
}

/// # SLR1 分析
/// ## 输入
/// - `g`: 文法
/// - `ACTION`: Action表
/// - `GOTO`: Goto表
/// - `token`: 词法分析得到的token序列
/// ## 输出
/// - `true`: 分析成功
/// - `false`: 分析失败
pub fn slr1_analysis(
    g: &Grammar,
    ACTION: &Vec<HashMap<String, String>>,
    GOTO: &Vec<HashMap<String, String>>,
    tokens: Vec<Token>,
) -> bool {
    // 初始化状态栈和符号栈
    let mut state_stack = vec![0];
    let mut symbol_stack = vec!["#".to_string()];

    // 输入缓冲区
    let mut buffer = tokens
        .into_iter()
        .map(|token| match token.token_type {
            TokenType::Identifier => "id".to_string(),
            TokenType::Constant => "value".to_string(),
            _ => token.token_value,
        })
        .collect::<VecDeque<String>>();
    buffer.push_back("#".to_string());
    debug!("init buffer: {:?}", buffer);

    let mut step = 1;
    loop {
        debug!(
            "step {}: \nstate_stack: {:?}\nsymbol_stack: {:?}\nbuffer: {:?}",
            step, state_stack, symbol_stack, buffer
        );
        step += 1;
        // 获取状态栈栈顶元素
        let state = state_stack.last().unwrap();
        // 获取输入缓冲区第一个元素
        let token = match buffer.front() {
            Some(token) => token,
            None => {
                error!("输入缓冲区为空");
                return false;
            }
        };
        // 获取ACTION表中的状态
        let action = match ACTION[*state].get(token) {
            Some(action) => action,
            None => {
                error!("ACTION表中没有状态({}, {})", state, token);
                return false;
            }
        };
        debug!("state: {}, token: {}, action: {:?}", state, token, action);
        // 如果是移进
        if action.starts_with("s") {
            debug!(
                "移进: 将 {} 状态压入状态栈，将 {} 符号压入符号栈",
                action, token
            );
            // 将状态压入状态栈
            state_stack.push(action[1..].parse::<usize>().unwrap());
            // 将输入缓冲区第一个元素压入符号栈
            symbol_stack.push(buffer.pop_front().unwrap());
        }
        // 如果是规约
        else if action.starts_with("r") {
            // 获取产生式
            let k = action[1..].parse::<usize>().unwrap();
            let p = &g.p[k];
            debug!("规约: 按照第{}个产生式 {} 进行规约", k, p);
            // 弹出状态栈中与产生式右部长度相同的元素
            for _ in 0..p.right.len() {
                state_stack.pop();
                symbol_stack.pop();
            }
            // 将产生式左部压入符号栈
            symbol_stack.push(p.left.clone());
            // 获取GOTO表中的状态
            let s = state_stack.last().unwrap();
            let state = GOTO[*s].get(&p.left).unwrap();
            // 将状态压入状态栈
            state_stack.push(state.parse::<usize>().unwrap());
        }
        // 如果是接受
        else if action == "acc" {
            debug!("接受");
            return true;
        }
        // 如果是错误
        else {
            error!("错误");
            return false;
        }
    }
}

pub fn slr1_analysis_with_log(
    g: &Grammar,
    ACTION: &Vec<HashMap<String, String>>,
    GOTO: &Vec<HashMap<String, String>>,
    tokens: Vec<Token>,
) -> bool {
    // 初始化状态栈和符号栈
    let mut state_stack = vec![0];
    let mut symbol_stack = vec!["#".to_string()];

    // 输入缓冲区
    let mut buffer = tokens
        .into_iter()
        .map(|token| match token.token_type {
            TokenType::Identifier => "id".to_string(),
            TokenType::Constant => "value".to_string(),
            _ => token.token_value,
        })
        .collect::<VecDeque<String>>();
    buffer.push_back("#".to_string());
    info!("init buffer: {:?}", buffer);

    let mut step = 1;
    loop {
        info!("-----step {}-----", step);
        info!("state_stack: {:?}", state_stack);
        info!("symbol_stack: {:?}", symbol_stack);
        info!("buffer: {:?}", buffer);
        step += 1;
        // 获取状态栈栈顶元素
        let state = state_stack.last().unwrap();
        // 获取输入缓冲区第一个元素
        let token = match buffer.front() {
            Some(token) => token,
            None => {
                error!("输入缓冲区为空");
                return false;
            }
        };
        // 获取ACTION表中的状态
        let action = match ACTION[*state].get(token) {
            Some(action) => action,
            None => {
                error!("ACTION表中没有状态({}, {})", state, token);
                return false;
            }
        };
        info!("state: {}, token: {}, action: {:?}", state, token, action);
        // 如果是移进
        if action.starts_with("s") {
            info!(
                "移进: 将 {} 状态压入状态栈，将 {} 符号压入符号栈",
                action, token
            );
            // 将状态压入状态栈
            state_stack.push(action[1..].parse::<usize>().unwrap());
            // 将输入缓冲区第一个元素压入符号栈
            symbol_stack.push(buffer.pop_front().unwrap());
        }
        // 如果是规约
        else if action.starts_with("r") {
            // 获取产生式
            let k = action[1..].parse::<usize>().unwrap();
            let p = &g.p[k];
            info!("规约: 按照第{}个产生式 {} 进行规约", k, p);
            // 弹出状态栈中与产生式右部长度相同的元素
            for _ in 0..p.right.len() {
                state_stack.pop();
                symbol_stack.pop();
            }
            info!("弹出{}个状态栈和符号栈中的元素", p.right.len());
            info!("state_stack: {:?}", state_stack);
            info!("symbol_stack: {:?}", symbol_stack);
            // 获取GOTO表中的状态
            let s = state_stack.last().unwrap();
            let state = GOTO[*s].get(&p.left).unwrap();
            info!(
                "查询GOTO表: 当前状态为 {} 时,接收到 {} 应当跳转到 {} 状态。",
                s, p.left, state
            );
            // 将产生式左部压入符号栈
            symbol_stack.push(p.left.clone());
            // 将状态压入状态栈
            state_stack.push(state.parse::<usize>().unwrap());
            info!("将 {} 状态压入状态栈，将 {} 符号压入符号栈", state, p.left);
            info!("state_stack: {:?}", state_stack);
            info!("symbol_stack: {:?}", symbol_stack);
        }
        // 如果是接受
        else if action == "acc" {
            info!("接受");
            return true;
        }
        // 如果是错误
        else {
            error!("错误");
            return false;
        }
    }
}

pub fn get_first(g: &Grammar) -> HashMap<String, Vec<String>> {
    let mut first = HashMap::new();
    // 终结符的 first 集合为自身
    g.t.iter().for_each(|t| {
        first.insert(t.clone(), vec![t.clone()]);
    });

    // 初始化非终结符的 first 集合为空
    g.v.iter().for_each(|v| {
        first.insert(v.clone(), vec![]);
    });

    // 若产生式右部第一个符号为终结符或右部只有有ε，则将其加入该非终结符的 first 集合
    g.p.iter().for_each(|p| {
        if g.t.contains(&p.right[0]) {
            first.get_mut(&p.left).unwrap().push(p.right[0].clone());
        }
        if p.right == vec!["ε".to_string()] {
            first.get_mut(&p.left).unwrap().push("ε".to_string());
        }
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

/// # 将 y first 集合中的终结符添加到 x first 集合中
///
/// - @param discard 是否丢弃 y first 集合中的 ε
/// - @return 是否发生了变化
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

pub fn get_follow(g: &Grammar) -> HashMap<String, Vec<String>> {
    let mut first = get_first(g);
    get_follow_with_first(g, &mut first)
}

pub fn get_follow_with_first(
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

                    // 否则将FITST(β)去除ε加入FOLLOW(B)
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

/// LR(0)项目
///
/// 一个LR(0)项目是带圆点的产生式
/// 项目的形式为 A -> α·Bβ
#[derive(Debug, Clone, PartialEq)]
struct Item {
    /// 产生式左部
    left: String,
    /// 产生式右部
    right: Vec<String>,
    /// ·的位置，在对应坐标字符的左边
    dot: usize,
}

/// # 求LR(0)项目集族
///
/// 每个项目集都是一个状态，项目集族就是所有状态的集合
///
/// 即求出识别过程中的所有状态
fn get_lr0_collection(g: &Grammar) -> Vec<Vec<Item>> {
    // 项目集规范族，所有状态的集合
    let mut c = vec![];

    // 开始项目集(状态)，将 S' -> ·S 加入到项目集族中
    let mut i = vec![];
    let first_prodution = g.p.iter().find(|p| p.left == g.s).unwrap();
    i.push(Item {
        left: first_prodution.left.clone(),
        right: first_prodution.right.clone(),
        dot: 0,
    });

    // 将开始项目集的完整表达加入到项目集规范族中
    c.push(closure(&i, g));

    // 终结符集和非终结符集
    let v_t =
        g.v.iter()
            .chain(g.t.iter())
            .cloned()
            .collect::<Vec<String>>();

    // 用于记录还未处理的项目集(状态)，相当于队列
    // 这里的处理是指求项目集(状态)接受任意终结符或非终结符能转移到的其他项目集(状态)
    let mut e = c.clone().into_iter().collect::<VecDeque<Vec<Item>>>();

    while e.len() > 0 {
        // 取出一个项目集
        let items = e.pop_front().unwrap();
        // 对于每个终结符或非终结符 x
        v_t.iter().for_each(|x| {
            // 求项目集 IT 在接受符号 x 时转移到的项目集
            let to_items = goto(&items, x, g);
            if to_items.len() > 0 {
                // 如果项目集 to_items 不在 C 中
                if !c.contains(&to_items) {
                    // 将 to_items 加入到 C 中
                    c.push(to_items.clone());
                    // 将 to_items 加入到未处理的项目集 E 中
                    e.push_back(to_items);
                }
            }
        });
    }

    c
}

/// # 项目集的状态转移函数
///
/// 求解项目集 I 接受 x 后转移到的项目集 J
///
/// 找到项目集中形如 A -> α·xβ 的项目，将 A -> αx·β 加入到 J 中
///
/// 然后求J的完整表示，即求闭包
fn goto(items: &Vec<Item>, x: &str, g: &Grammar) -> Vec<Item> {
    let mut j = vec![];

    items.iter().for_each(|item| {
        if item.dot >= item.right.len() {
            return;
        }
        let a = &item.right[item.dot];
        // 找到形如 A -> α·xβ 的项目
        if a == x && (g.v.contains(a) || g.t.contains(a)) {
            // 将 A -> αx·β 加入到 J 中
            let mut new_item = item.clone();
            new_item.dot += 1;
            j.push(new_item);
        }
    });

    // 求闭包，即求出该状态的完整表达
    closure(&j, g)
}

/// # 在拓广文法G'中求解项目集I的闭包J
///
/// 闭包的定义为：J = I U {B -> ·γ | A -> α·Bβ ∈ J, B -> γ ∈ G'}
///
/// 即完善项目集I中的状态，将非终结符展开，找出下一步能接受的终结符
///
/// 可以理解为求出项目集I的完整表达，便于求出下一步能接受的终结符
fn closure(i: &[Item], g: &Grammar) -> Vec<Item> {
    // 用于存储闭包
    let mut j = i.to_vec();
    // 模拟队列，用于存储还未处理的项目
    let mut e = i.to_vec().into_iter().collect::<VecDeque<Item>>();

    while e.len() > 0 {
        // 取出队列中的第一个项目
        let item = e.pop_front().unwrap();
        if item.dot < item.right.len() {
            // 获取圆点后面的第一个单词
            let a = &item.right[item.dot];

            // 如果是终结符，则跳过
            if !g.v.contains(a) {
                continue;
            }

            // 若为非终结符，遍历所有产生式，找到左部为a的产生式
            g.p.iter().filter(|p| p.left == *a).for_each(|p| {
                // 将产生式加入到闭包中
                let new_item = Item {
                    left: p.left.clone(),
                    right: p.right.clone(),
                    dot: 0,
                };
                if !j.contains(&new_item) {
                    j.push(new_item.clone());
                    e.push_back(new_item);
                }
            });
        }
    }

    j
}

/// # 对比两个项目集是否相同
///
/// 当两个项目集长度相同且对一个项目集中的每个项目都能在另一个项目集中找到对应的项目时，两个项目集相同
fn items_eq(items1: &Vec<Item>, items2: &Vec<Item>) -> bool {
    if items1.len() != items2.len() {
        return false;
    }

    for i in items1 {
        if !items2.contains(i) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use log::info;
    use simplelog::*;

    use super::Grammar;
    use crate::parser::{
        lexical_analysis::lexical_analysis,
        syntax_analysis::{get_first, get_follow, get_slr1_table, slr1_analysis},
    };

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

    const GRAMMAR_YML: &str = r#"
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
        let g = Grammar::from_yml(GRAMMAR_YML).unwrap();
        println!("{:#?}", g);
        let mut first = get_first(&g);
        println!("{:#?}", first);
        first.iter_mut().for_each(|(_k, v)| {
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
        let g = Grammar::from_yml(GRAMMAR_YML).unwrap();
        println!("{:#?}", g);
        let mut follow = get_follow(&g);
        println!("{:#?}", follow);
        follow.iter_mut().for_each(|(_k, v)| {
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

    const PROGRAM: &str = r#"
    // This is a note.
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
    }
    "#;

    #[test]
    fn test_slr1_analysis() {
        CombinedLogger::init(vec![
            TermLogger::new(
                LevelFilter::Debug,
                Config::default(),
                TerminalMode::Mixed,
                ColorChoice::Auto,
            ),
            WriteLogger::new(
                LevelFilter::Debug,
                Config::default(),
                File::create("slr1.log").unwrap(),
            ),
        ])
        .unwrap();

        let (tokens, _success) = lexical_analysis(PROGRAM.to_string()).unwrap();
        info!("tokens:");
        for token in tokens.iter() {
            info!(
                "value: \"{}\", type: {}",
                token.token_value, token.token_type
            );
        }

        let yml = std::fs::read_to_string("grammar.yml").unwrap();
        let g = Grammar::from_yml(&yml).unwrap();
        match g.validate() {
            Ok(_) => {}
            Err(_) => panic!("grammar is not valid"),
        }
        info!("grammar:");
        info!("s: {}", g.s);
        info!("v: {:?}", g.v);
        info!("t: {:?}", g.t);
        info!("p:");
        for p in g.p.iter() {
            info!(
                "  \"{}\" -> {}",
                p.left,
                p.right
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }

        let mut first = get_first(&g);
        first.iter_mut().for_each(|(_k, v)| {
            v.sort();
        });
        info!("first:");
        for (k, v) in first.iter() {
            info!("FIRST(\"{}\") = {:?}", k, v);
        }

        let mut follow = get_follow(&g);
        follow.iter_mut().for_each(|(_k, v)| {
            v.sort();
        });
        info!("follow:");
        for (k, v) in follow.iter() {
            info!("FOLLOW(\"{}\") = {:?}", k, v);
        }

        let (action, goto) = match get_slr1_table(&g) {
            Ok((action, goto)) => (action, goto),
            Err(e) => panic!("get slr1 table failed: {}", e),
        };
        info!("action:");
        let mut buffer = String::new();
        buffer.push_str(&format!("{:<6}", ""));
        for t in &g.t {
            buffer.push_str(&format!("{:<6}", t));
        }
        buffer.push_str(&format!("{:<6}", "#"));
        info!("{}", buffer);
        buffer.clear();
        for (i, map) in action.iter().enumerate() {
            buffer.push_str(&format!("{:<6}", i));
            for t in &g.t {
                if let Some(act) = map.get(t) {
                    buffer.push_str(&format!("{:<6}", act));
                } else {
                    buffer.push_str(&format!("{:<6}", ""));
                }
            }
            if let Some(act) = map.get("#") {
                buffer.push_str(&format!("{:<6}", act));
            } else {
                buffer.push_str(&format!("{:<6}", ""));
            }
            info!("{}", buffer);
            buffer.clear();
        }
        info!("goto:");
        buffer.push_str(&format!("{:<10}", ""));
        for nt in &g.v {
            buffer.push_str(&format!("{:<10}", nt));
        }
        info!("{}", buffer);
        buffer.clear();
        for (i, map) in goto.iter().enumerate() {
            buffer.push_str(&format!("{:<10}", i));
            for nt in &g.v {
                if let Some(act) = map.get(nt) {
                    buffer.push_str(&format!("{:<10}", act));
                } else {
                    buffer.push_str(&format!("{:<10}", ""));
                }
            }
            info!("{}", buffer);
            buffer.clear();
        }

        let slr1 = slr1_analysis(&g, &action, &goto, tokens);
        info!("slr1 success: {:?}", slr1);
        assert!(slr1);
    }
}
