#![deny(unused_crate_dependencies)]

pub mod error_template;
pub mod errors;

#[cfg(feature = "csr")]
pub mod client;
pub mod game;
pub mod server;

#[cfg(feature = "csr")]
#[cfg_attr(feature = "csr", wasm_bindgen::prelude::wasm_bindgen)]
pub fn hydrate() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount_to_body(crate::client::app_root::AppRoot);
}

#[cfg(not(feature = "csr"))]
pub fn hydrate() {}
