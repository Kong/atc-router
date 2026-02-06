use atc_router::parser;
use std::fs::read_dir;

fn main() {
    for file in read_dir("fuzz/corpus/compare_parser").unwrap() {
        let file = file.unwrap();
        let path = file.path();
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let content = content
            .replace('\n', " ")
            .replace('\t', " ")
            .replace('\r', " ");
        let result = parser::parse(&content);
        if let Err(e) = result {
            println!("{}", content);
            println!("------");
            println!("{}", e);
            println!();
        }
    }
}
