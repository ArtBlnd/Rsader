fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=src/ast/parse_rule.lalrpop");

    lalrpop::Configuration::new()
        .process_file("./src/ast/parse_rule.lalrpop")
        .unwrap();
}
