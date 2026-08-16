use std::fs;
use crawfish::lexical_analysis::tokenizer::Tokenizer;
use std::path::PathBuf;

fn main() {
    let input = fs::read_to_string(PathBuf::from("simple.crw")).unwrap();
    let mut tokenizer = Tokenizer::new(&input);
    let tokens = tokenizer.tokenize();
}
