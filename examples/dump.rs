//! Write a plain-text (RON) view of a DWG: model-space entities, decoded
//! alignments and a count of raw records; `--raw` adds every raw record's
//! bytes in hex (D-065, `src/dump.rs`).
//!
//! Usage: cargo run --release --example dump -- file.dwg [out.ron] [--raw]
//! Without out.ron the dump goes to standard output.

use civil3drust::{dump, raw};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let with_raw = args.iter().any(|a| a == "--raw");
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let Some(path) = files.first() else {
        eprintln!("usage: dump <file.dwg> [out.ron] [--raw]");
        std::process::exit(2);
    };
    let opened = raw::open(path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });
    if let Some(r) = &opened.failsafe_reason {
        eprintln!("note: read in failsafe mode; some records were dropped ({r})");
    }
    let name = Path::new(path.as_str())
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    let text = dump::to_ron(&dump::dump(&opened, &name, with_raw));
    match files.get(1) {
        Some(out) => {
            std::fs::write(out.as_str(), &text).unwrap_or_else(|e| {
                eprintln!("failed to write {out}: {e}");
                std::process::exit(1);
            });
            eprintln!("wrote {out} ({} bytes)", text.len());
        }
        None => print!("{text}"),
    }
}
