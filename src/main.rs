use std::fs::File;

use log::{info, LevelFilter};
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode, WriteLogger};

use crate::parser::{
    lexical_analysis::lexical_analysis,
    syntax_analysis::{get_first, get_follow, get_slr1_table, slr1_analysis_with_log, Grammar},
};

mod parser;

fn main() {
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

    let program = std::fs::read_to_string("program.txt").expect("Unable to read file program.txt");

    let (tokens, _success) = lexical_analysis(program.to_string()).unwrap();
    info!("tokens:");
    for token in tokens.iter() {
        info!(
            "value: \"{}\", type: {}",
            token.token_value, token.token_type
        );
    }

    let yml = std::fs::read_to_string("grammar.yml").expect("Unable to read file grammar.yml");
    let g = Grammar::from_yml(&yml).unwrap();
    match g.validate() {
        Ok(_) => {}
        Err(e) => panic!("grammar is not valid: {}", e),
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

    let (action, goto) = get_slr1_table(&g);
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

    let slr1 = slr1_analysis_with_log(&g, &action, &goto, tokens);
    info!("slr1 success: {:?}", slr1);
}
