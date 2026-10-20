//! Build script: guarantees that `web/dist` exists before `rust-embed` reads it.
//!
//! The front is built with Node (`pnpm -C web build`), which a Rust-only build,
//! `cargo test` or the CI's Rust job does not run. When `web/dist/index.html` is
//! missing, a placeholder page is written there so the crate always compiles and
//! `ariane serve` explains how to get the real interface (ADR-0012).

use std::path::Path;

/// The page served when the front has not been built.
const PLACEHOLDER: &str = "<!doctype html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\"><title>Ariane</title></head>\n<body><p>The front is not built: run <code>pnpm -C web build</code>, then rebuild <code>ariane</code>.</p></body>\n</html>\n";

/// Writes the placeholder when needed and asks cargo to rerun when `web/dist` changes.
fn main() {
    let dist = Path::new("web/dist");
    let entry = dist.join("index.html");
    println!("cargo:rerun-if-changed=web/dist");
    if !entry.exists() {
        std::fs::create_dir_all(dist).expect("creating web/dist");
        std::fs::write(&entry, PLACEHOLDER).expect("writing the placeholder page");
    }
}
