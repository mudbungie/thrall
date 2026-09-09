//! **The vendored wire version's one file-shaped home, compiled in**
//! (bl-c618).
//!
//! The repo-root `PROTOCOL` file states the integer this build speaks — one
//! line, nothing else — and this script turns it into the constant
//! `src/channel/hello/version.rs` includes. The file IS the source: there is
//! no second copy of the number to hold equal to it.
//!
//! **Why a file and not a Rust declaration.** This number is fetched by gates
//! that do not build this crate — yog's release gate reads it off this
//! repository's `main` before it will publish a bump (yog `docs/REMOTE.md`
//! §3), and this repository's own gate reads the engine's the same way — and a
//! Rust path is not a stable address for a fetch. yog split
//! `src/wire/hello.rs` into `src/wire/hello/version.rs` and left the old path
//! re-exporting: invisible to a build, unreadable to a regex, so this
//! repository's gate silently stopped being able to read the engine at all and
//! would have held its own release forever on the bump it was waiting for. A
//! top-level file with no extension is the one path a module split cannot
//! move, and every gate in all four repositories now reads it there.
//!
//! Errors here fail the build, which is the correct loudness: a `PROTOCOL`
//! file that is missing or is not one integer means nothing downstream can be
//! trusted to have read it either.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// Read the root `PROTOCOL` file and write the constant into `OUT_DIR`.
fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=PROTOCOL");
    let stated =
        fs::read_to_string(PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("PROTOCOL"))?;
    let protocol: u32 = stated.trim().parse().map_err(|_| {
        format!("the repo-root PROTOCOL file must state one integer; it states {stated:?}")
    })?;
    fs::write(
        PathBuf::from(env::var("OUT_DIR")?).join("protocol.rs"),
        format!(
            "/// The integer the repo-root `PROTOCOL` file states, compiled in by\n\
             /// `build.rs`. Re-exported, with the reasoning, as the module's\n\
             /// `PROTOCOL`.\n\
             pub const PROTOCOL: u32 = {protocol};\n"
        ),
    )?;
    Ok(())
}
