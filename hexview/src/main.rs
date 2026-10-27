//! civil3drust hexview: a hex editor-style view of a DWG with every object
//! record outlined and labelled by the library that decodes it.
//!
//! ```text
//! civil3drust-hexview [file.dwg] [--goto <hex offset|#handle>]
//!                                         open the window
//! civil3drust-hexview --dump file.dwg     list panes and records, then exit
//! ```
//!
//! Colors: blue = acadrust decodes the record, red = AECC record read by
//! civil3drust, amber = other record acadrust keeps opaque, gray = in the
//! handle map but missing from acadrust's document.

mod app;
mod model;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--dump") => {
            let Some(path) = args.get(1) else { usage() };
            dump(path);
        }
        Some("-h" | "--help") => usage(),
        _ => {
            let goto = args
                .iter()
                .position(|a| a == "--goto")
                .and_then(|i| args.get(i + 1).cloned());
            let path = args.first().filter(|a| *a != "--goto").cloned();
            if let Err(e) = app::run(path, goto) {
                eprintln!("hexview failed to start: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn usage() -> ! {
    eprintln!("usage: civil3drust-hexview [file.dwg] [--goto <hex offset|#handle>]\n       civil3drust-hexview --dump file.dwg");
    std::process::exit(2);
}

fn dump(path: &str) {
    let l = model::load(path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });
    println!("{} [{}]", l.path, l.version);
    for w in &l.warnings {
        println!("warning: {w}");
    }
    for p in &l.panes {
        let covered: usize = p.spans.iter().map(|s| s.len).sum();
        let bad_crc = p.spans.iter().filter(|s| !s.crc_ok).count();
        println!(
            "\n== {} ({} bytes, {} records covering {} bytes, {} CRC mismatches)",
            p.title,
            p.bytes.len(),
            p.spans.len(),
            covered,
            bad_crc
        );
        if p.title != "AcDb:AcDbObjects" {
            continue;
        }
        for s in &p.spans {
            println!(
                "{:>8X} {:>6} {:>8X} {:<9?} {}",
                s.start,
                s.len,
                s.handle,
                s.origin,
                s.label()
            );
        }
    }
}
