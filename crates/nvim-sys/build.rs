fn main() {
    nvim_bindgen::generate(
        concat!(env!("CARGO_MANIFEST_DIR"), "/", "./../../vendor/neovim/"),
        &format!("{}/nvim-bindings.rs", std::env::var("OUT_DIR").unwrap(),),
    );
}
