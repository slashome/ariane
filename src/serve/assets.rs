//! The front, embedded in the binary at build time (ADR-0012).
//!
//! `rust-embed` copies `web/dist` into the executable when it is compiled, so
//! `ariane serve` needs neither Node nor any file on disk to run. `build.rs`
//! writes a placeholder page there when the front has not been built, so the
//! crate always compiles.

use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// The files of `web/dist`, compiled into the binary.
#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Front;

/// The page every unknown path falls back to: the front's router takes it from there.
const ENTRY: &str = "index.html";

/// Serves a file of the front, or `index.html` for any path that is not one.
///
/// The fallback is what lets a deep link such as `/tasks?node=cmc` be reloaded
/// in the browser: the server answers the entry page and the front's own router
/// reads the address.
pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { ENTRY } else { path };
    match Front::get(path)
        .map(|file| (path, file))
        .or_else(|| Front::get(ENTRY).map(|file| (ENTRY, file)))
    {
        Some((served, file)) => {
            let mime = mime_guess::from_path(served).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref().to_owned())],
                file.data,
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            "the front is not embedded in this build",
        )
            .into_response(),
    }
}
