//! `c3dr`: every civil3drust command from the shell (D-074).
//!
//! ```text
//! c3dr help [command]                     list commands / one command's arguments
//! c3dr --spec                             the command list as RON (for GUIs)
//! c3dr <command> in.dwg [args...]         a reporting command (alignments, styles, ...)
//! c3dr <command> in.dwg out.dwg [args...] an editing command; out.dwg is written
//!                                         and read back
//! ```
//!
//! `in.dwg` and `out.dwg` may not be the same file, and out.dwg must not
//! be one of the sample drawings.

use civil3drust::commands::{self, quote};
use civil3drust::raw;
use std::path::Path;
use std::process::exit;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.first().map(String::as_str) {
        None | Some("-h" | "--help") => {
            println!(
                "usage: c3dr <command> in.dwg [out.dwg] [args...]\n       c3dr help [command]\n       c3dr --spec\n"
            );
            println!("{}", commands::help_text(None).unwrap());
            return;
        }
        Some("help") => match commands::help_text(a.get(1).map(String::as_str)) {
            Ok(t) => {
                println!("{t}");
                return;
            }
            Err(e) => fail(e),
        },
        Some("--spec") => {
            let cfg = ron::ser::PrettyConfig::new().depth_limit(3);
            println!(
                "{}",
                ron::ser::to_string_pretty(&commands::spec(), cfg).expect("spec serializes")
            );
            return;
        }
        _ => {}
    }
    let name = &a[0];
    let cmd = commands::find(name).unwrap_or_else(|| fail(format!("unknown command '{name}'")));
    let n_files = if cmd.edits { 2 } else { 1 };
    if a.len() < 1 + n_files {
        fail(format!(
            "usage: c3dr {} in.dwg{} {}",
            cmd.name,
            if cmd.edits { " out.dwg" } else { "" },
            cmd.usage().trim_start_matches(cmd.name).trim()
        ));
    }
    let input = &a[1];
    let output = cmd.edits.then(|| &a[2]);
    if let Some(out) = output {
        if Path::new(out) == Path::new(input)
            || std::fs::canonicalize(out).ok() == std::fs::canonicalize(input).ok()
                && Path::new(out).exists()
        {
            fail("out.dwg must differ from in.dwg");
        }
        if Path::new(out)
            .parent()
            .and_then(|p| p.file_name())
            .is_some_and(|n| n == "samples")
        {
            fail("refusing to write into samples/");
        }
    }
    let mut opened = raw::open(input).unwrap_or_else(|e| fail(format!("{input}: {e}")));
    if let Some(r) = &opened.failsafe_reason {
        eprintln!("WARNING: read in failsafe mode; unparsed records will be missing from the output. ({r})");
    }
    // Rebuild the line so run_line sees the arguments as typed.
    let rest: Vec<String> = a[1 + n_files..].iter().map(|s| quote(s)).collect();
    let line = format!("{} {}", cmd.name, rest.join(" "));
    match commands::run_line(&mut opened.doc, &line) {
        Ok(report) => println!("{report}"),
        Err(e) => fail(e),
    }
    if let Some(out) = output {
        acadrust::DwgWriter::write_to_file(out, &opened.doc).unwrap_or_else(|e| fail(e));
        raw::open(out).unwrap_or_else(|e| fail(format!("{out} does not read back: {e}")));
        println!("wrote {out}");
    }
}
