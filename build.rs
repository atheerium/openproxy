//! Build script.
//!
//! When the `embed-web` feature is enabled (default for release builds), the
//! Astro static output at `web/dist/` is baked into the binary via
//! `rust-embed`. This script fails the build early with a clear message if
//! `web/dist/index.html` is missing, instead of producing a binary with an
//! empty dashboard.
//!
//! To intentionally build without the embedded UI (smaller binary, requires
//! `--dashboard-sidecar-url` or `--web-dir` at runtime):
//!     cargo build --release --no-default-features

fn main() {
    let embed_enabled = std::env::var("CARGO_FEATURE_EMBED_WEB").is_ok();
    if !embed_enabled {
        return;
    }

    let dist = std::path::Path::new("web/dist/index.html");
    if !dist.exists() {
        // `cargo:warning=` lines are printed without colour but are visible in
        // release builds. We also panic so the build actually fails.
        println!(
            "cargo:warning=web/dist/index.html is missing. \
             Build the dashboard first: (cd web && pnpm install --frozen-lockfile && pnpm run build)"
        );
        panic!(
            "web/dist not built. Run:\n  \
             (cd web && pnpm install --frozen-lockfile && pnpm run build)\n\
             Or build without the embedded UI:\n  \
             cargo build --release --no-default-features"
        );
    }

    // Trigger a rebuild whenever the embedded assets change. Without this,
    // editing `web/dist/...` won't invalidate the existing rust-embed cache
    // and the binary will keep serving stale assets.
    println!("cargo:rerun-if-changed=web/dist");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_EMBED_WEB");

    // Guard: when building the embedded (release) dashboard, refuse to bake a
    // `web/dist/` that is OLDER than the source `web/src/`. Stale embedding is
    // the classic "blank /dashboard/providers, chunk ReferenceError" failure:
    // a developer edits web/src, rebuilds the binary WITHOUT rerunning
    // `pnpm build`, and ships a dashboard predating their fix. Dev (debug)
    // builds are exempt so the normal edit-loop isn't blocked.
    let is_release = std::env::var("PROFILE")
        .map(|p| p == "release")
        .unwrap_or(false);
    if is_release {
        if let Some(msg) = newest_src_newer_than_dist() {
            println!(
                "cargo:warning=web/src is newer than web/dist — embedded dashboard will be STALE. Rebuild it: (cd web && pnpm install && pnpm run build)"
            );
            panic!(
                "web/src is newer than web/dist ({msg}).\n  \
                 The embedded dashboard would be stale and the running server\n  \
                 would serve an old chunk (blank page / ReferenceError).\n  \
                 Fix: (cd web && pnpm install && pnpm run build) then rebuild.\n  \
                 To skip the embed entirely: cargo build --release --no-default-features"
            );
        }
    }
}

/// Returns `Some(reason)` if any file under `web/src` has an mtime strictly
/// newer than `web/dist/index.html` (the build output marker). Walks `web/src`
/// recursively; tolerates missing source dir.
fn newest_src_newer_than_dist() -> Option<String> {
    let dist_marker = std::path::Path::new("web/dist/index.html");
    let dist_mtime = match std::fs::metadata(dist_marker).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return Some("web/dist/index.html missing".to_string()),
    };
    let src_dir = std::path::Path::new("web/src");
    if !src_dir.exists() {
        return None;
    }
    let mut newest_src: Option<std::time::SystemTime> = None;
    let mut stack = vec![src_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let md = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.is_dir() {
                stack.push(path);
            } else if let Ok(mt) = md.modified() {
                newest_src = Some(match newest_src {
                    Some(n) => n.max(mt),
                    None => mt,
                });
            }
        }
    }
    match newest_src {
        Some(src_mt) if src_mt > dist_mtime => Some(format!(
            "src mtime {:?} > dist mtime {:?}",
            src_mt, dist_mtime
        )),
        _ => None,
    }
}
