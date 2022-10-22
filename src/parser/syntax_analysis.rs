pub struct Product {
    pub left: String,       // 产生式左部，为一个非终结符
    pub right: Vec<String>, // 产生式右部，含多个终结符或非终结符
}

pub struct Grammar {
    pub s: String,       // 开始符号
    pub v: Vec<String>,  // 非终结符集
    pub t: Vec<String>,  // 终结符集
    pub p: Vec<Product>, // 产生式集
}

pub fn syntax_analysis() {
    // #[rustfmt::skip]
    let g = Grammar {
        s: "程序".to_string(),
        v: vec![
            "程序".to_string(),
            "函数定义".to_string(),
            "形式参数".to_string(),
            "代码块".to_string(),
            "变量类型".to_string(),
            "算术表达式".to_string(),
            "布尔表达式".to_string(),
            "比较运算符".to_string(),
            "算术运算符".to_string(),
        ],
        t: vec![],
        p: vec![],
    };
}
