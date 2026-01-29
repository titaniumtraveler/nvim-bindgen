fn main() {
    nvim_codegen::generate(
        concat!(env!("CARGO_MANIFEST_DIR"), "/", "./../../vendor/neovim/"),
        &format!("{}/nvim-bindings.rs", std::env::var("OUT_DIR").unwrap(),),
    );
}
