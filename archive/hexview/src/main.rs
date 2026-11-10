//! civil3drust hexview: a hex editor-style view of a DWG with every object
//! record outlined and labelled by the library that decodes it.
//!
//! ```text
//! civil3drust-hexview [file.dwg] [--goto <hex offset|#handle>] [--compare other.dwg]
//!                                         open the window
//! civil3drust-hexview --dump file.dwg     list panes and records, then exit
//! civil3drust-hexview --diff a.dwg b.dwg [--show civil,opaque,unmapped,acad|all] [--filter text]
//!                                         print the comparison report
//! ```
//!
//! Colors: blue = acadrust decodes the record, red = AECC record read by
//! civil3drust, amber = other record acadrust keeps opaque, gray = in the
//! handle map but missing from acadrust's document.

mod app;
mod compare;
mod model;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--dump") => {
            let Some(path) = args.get(1) else { usage() };
            dump(path);
        }
        Some("--diff") => {
            let (Some(a), Some(b)) = (args.get(1), args.get(2)) else {
                usage()
            };
            let show = flag(&args, "--show").unwrap_or_else(|| "civil,opaque,unmapped".into());
            let filter = flag(&args, "--filter").unwrap_or_default();
            diff(a, b, &show, &filter);
        }
        Some("-h" | "--help") => usage(),
        _ => {
            let goto = flag(&args, "--goto");
            let compare_with = flag(&args, "--compare");
            let path = args.first().filter(|a| !a.starts_with("--")).cloned();
            if let Err(e) = app::run(path, goto, compare_with) {
                eprintln!("hexview failed to start: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn usage() -> ! {
    eprintln!(
        "usage: civil3drust-hexview [file.dwg] [--goto <hex offset|#handle>] [--compare other.dwg]\n       \
         civil3drust-hexview --dump file.dwg\n       \
         civil3drust-hexview --diff a.dwg b.dwg [--show civil,opaque,unmapped,acad|all] [--filter text]"
    );
    std::process::exit(2);
}

fn diff(a: &str, b: &str, show: &str, filter: &str) {
    let load = |p: &str| {
        model::load(p).unwrap_or_else(|e| {
            eprintln!("failed to read {p}: {e}");
            std::process::exit(1);
        })
    };
    let (la, lb) = (load(a), load(b));
    let c = compare::compare(&la, &lb);
    let want: Vec<&str> = show.split(',').map(str::trim).collect();
    let show = |o: model::Origin| {
        let name = match o {
            model::Origin::Acad => "acad",
            model::Origin::Civil => "civil",
            model::Origin::Opaque => "opaque",
            model::Origin::Unmapped => "unmapped",
        };
        want.contains(&name) || want.contains(&"all")
    };
    let text = compare::report(&c, &la, &lb, &show, filter);
    // Ignore a closed pipe (`| head`).
    let _ = std::io::Write::write_all(&mut std::io::stdout(), text.as_bytes());
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
