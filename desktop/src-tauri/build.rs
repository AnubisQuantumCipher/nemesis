use std::env;
use std::fs;
use std::path::PathBuf;

/// NativePaint fail-closed guard.
///
/// Production builds (`tauri build`, which enables tauri's `custom-protocol`
/// feature and therefore turns `tauri_build::is_dev()` false) embed `../dist`
/// into the binary at compile time through `tauri::generate_context!`. If the
/// built frontend is absent or hollow at that moment, Tauri embeds nothing and
/// the shipped webview falls back to a dev URL that is dead in production — a
/// blank window. Refuse to compile instead of shipping a blank app.
///
/// Dev-context builds (`tauri dev`, plain `cargo build`) intentionally skip
/// the guard: they load the live dev server and never embed assets.
fn enforce_frontend_dist() {
    if tauri_build::is_dev() {
        return;
    }
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("cargo always sets CARGO_MANIFEST_DIR"),
    );
    let index = manifest_dir.join("../dist/index.html");
    println!("cargo:rerun-if-changed={}", index.display());
    let contents = fs::read_to_string(&index).unwrap_or_else(|error| {
        panic!(
            "FAIL_FRONTEND_DIST_MISSING: production build requires the built frontend at {} \
             (run `npm run build` first): {error}",
            index.display()
        )
    });
    if !contents.contains("<div id=\"root\">") {
        panic!(
            "FAIL_FRONTEND_DIST_HOLLOW: {} exists but has no `<div id=\"root\">` mount point; \
             the webview would paint blank. Rebuild the frontend with `npm run build`.",
            index.display()
        );
    }
}

fn main() {
    enforce_frontend_dist();
    tauri_build::build();
}
