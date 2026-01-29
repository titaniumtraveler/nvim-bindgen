#![allow(
    nonstandard_style,
    unnecessary_transmutes,
    clippy::too_many_arguments,
    clippy::useless_transmute,
    clippy::missing_safety_doc,
    clippy::ptr_offset_with_cast
)]

include!(concat!(env!("OUT_DIR"), "/nvim-bindings.rs"));
