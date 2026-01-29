# nvim-bindgen

Very work-in-progress attempt to generate rust bindings for `nvim`.

__Use at your own risk!__

## Getting Started

Note: These steps were only tested on linux.
Also if you have problems in step 2 with building `neovim`, try to consulting `./vendor/neovim/BUILD.md`

1. Clone the project to your local computer using `git clone --recurse-submodules https://github.com/titaniumtraveler/nvim-bindgen`
2. `cd nvim-bindgen/vendor/neovim/` into the neovim submodule
  and build it via `make CMAKE_BUILD_TYPE=RelWithDebInfo`
3. `cargo add --path <path-to-nvim-bindgen>/crates/nvim-sys` to your project
4. Add the following to your `Cargo.toml` to make it build a dynamic library
```toml
[lib]
name = "projectname"
crate-type = ["cdylib"]
```
5. Add a plugin-entry function for `lua` to load like this:
```rust
#![allow(non_snake_case)] // all the lua functions use `L` for the `lua_State`

use nvim_sys::*;

#[unsafe(no_mangle)]
unsafe extern "C" fn luaopen_libprojectname(L: *mut lua_State) -> i32 {
  lua_settop(L, 0);
  0
}
```
6. `cargo build` your library, `cd target/debug` and make lua load your library
   using `nvim '+lua =require "libprojectname"'`
7. Profit!
