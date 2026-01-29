use crate::{cdoc_parser::FormatComment, ref_write::RefWrite};
use bindgen::{Function, ItemKind, Var, callbacks::ParseCallbacks, ir::item::Item};
use std::{
    fmt::Debug,
    io::{self, Write},
};

pub mod cdoc_parser;
pub mod ref_write;

#[derive(Debug)]
struct Callback<W>
where
    W: Debug,
{
    #[allow(dead_code)]
    writer: W,
}

impl<W> Callback<W>
where
    W: Write,
    W: Debug,
{
    fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W> ParseCallbacks for Callback<W>
where
    W: Debug + RefWrite,
{
    fn process_comment(&self, comment: &str) -> Option<String> {
        Some(FormatComment(comment).to_string())
    }

    fn do_include_root_item(&self, item: &Item) -> bool {
        match &item.kind {
            ItemKind::Function(Function { name, .. }) => {
                (name.starts_with("nvim_") & !name.starts_with("nvim__"))
                    || name.starts_with("arena_")
                    || name == "free_block"
                    || name.starts_with("lua_")
                    || name.starts_with("luaJIT_")
            }
            ItemKind::Var(Var { name, .. }) => {
                name.starts_with("LUA_") | name.starts_with("LUAJIT_")
            }
            _ => false,
        }
    }
}

pub fn generate(nvim_dir: &str, out_file: &str) {
    let callback = Callback::new(io::sink());
    let mut bindings_builder = bindgen::builder()
        .header(concat!(env!("CARGO_MANIFEST_DIR"), "/wrapper.h"))
        .parse_callbacks(Box::new(callback))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    bindings_builder = bindings_builder
        .clang_arg("-DMAKE_LIB")
        .clang_arg("-DUTF8PROC_STATIC")
        .clang_arg("-D_GNU_SOURCE");

    let flag = "-I";
    let dirs = [
        "build/src/nvim/auto",
        "build/include",
        "build/cmake.config",
        "src",
    ];

    let mut buf = String::with_capacity(
        flag.len() + nvim_dir.len() + dirs.into_iter().map(str::len).max().unwrap_or(0),
    );

    bindings_builder = bindings_builder.clang_args(dirs.iter().map(|dir| {
        buf.clear();
        buf.push_str(flag);
        buf.push_str(nvim_dir);
        buf.push_str(dir);
        buf.clone()
    }));

    bindings_builder = bindings_builder.merge_extern_blocks(true);

    bindings_builder
        .generate()
        .unwrap()
        .write_to_file(out_file)
        .unwrap();
}
