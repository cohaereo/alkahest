#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case)]
#![allow(
    clippy::missing_safety_doc,
    clippy::too_many_arguments,
    clippy::doc_markdown,
    clippy::semicolon_if_nothing_returned,
    clippy::use_self
)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// cohae: link_cplusplus needs to be referenced as external crate in order to link stdc++
#[allow(unused_extern_crates)]
extern crate link_cplusplus;

// pub mod query;
// pub mod tome;
