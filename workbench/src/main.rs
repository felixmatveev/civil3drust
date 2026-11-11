//! civil3drust workbench: open a DWG, look at it, inspect objects as hex
//! and RON, edit them with commands, save a copy (D-073, D-074).
//!
//! ```text
//! civil3drust-workbench [file.dwg] [--goto <handle>] [--compare other.dwg]
//!                                                  open the window
//! civil3drust-workbench --script file.dwg [--save out.dwg] "command" ...
//!                                                  run commands headless, print the log
//! civil3drust-workbench --related file.dwg <handle> [depth]
//!                                                  list the records the hex pane shows
//! civil3drust-workbench --objects file.dwg          list the drawn objects (viewer --dump)
//! civil3drust-workbench --dump file.dwg             list sections and records (hexview --dump)
//! civil3drust-workbench --diff a.dwg b.dwg [--show civil,opaque,unmapped,acad|all] [--filter text]
//!                                                  record-by-record bit diff (hexview --diff)
//! civil3drust-workbench --svg out.svg file.dwg [options]
//!     --select <handle> --3d --z-scale <n> --no-proxy --no-decoded --placeholders
//!     --no-context --light --size WxH               render the view to SVG
//! ```

mod app;
mod compare;
mod filemap;
mod fileview;
mod hex;
mod scene;
mod state;
mod svg;
mod view;

use civil3drust::commands::parse_handle;
use state::{LogKind, Relation, Workbench};
use std::process::exit;

fn usage() -> ! {
    eprintln!(
        "usage: civil3drust-workbench [file.dwg] [--goto <handle>] [--compare other.dwg]\n       \
         civil3drust-workbench --script file.dwg [--save out.dwg] \"command\" ...\n       \
         civil3drust-workbench --related file.dwg <handle> [depth]\n       \
         civil3drust-workbench --objects file.dwg\n       \
         civil3drust-workbench --dump file.dwg\n       \
         civil3drust-workbench --diff a.dwg b.dwg [--show civil,opaque,unmapped,acad|all] [--filter text]\n       \
         civil3drust-workbench --svg out.svg file.dwg [--select <handle>] [--3d] [--z-scale <n>] [--no-proxy] \
         [--no-decoded] [--placeholders] [--no-context] [--light] [--size WxH]"
    );
    exit(2);
}

/// `println!` into a String.
macro_rules! outln {
    ($out:expr, $($t:tt)*) => {{
        use std::fmt::Write as _;
        let _ = writeln!($out, $($t)*);
    }};
}

fn open_or_exit(path: &str) -> Workbench {
    Workbench::open(path).unwrap_or_else(|e| {
        eprintln!("failed to open {path}: {e}");
        exit(1);
    })
}

fn print_log(wb: &Workbench, from: usize) {
    for (k, line) in &wb.log[from..] {
        let tag = match k {
            LogKind::Command => ">",
            LogKind::Ok => " ",
            LogKind::Error => "!",
            LogKind::Info => "#",
        };
        for l in line.lines() {
            println!("{tag} {l}");
        }
    }
}

fn script(args: &[String]) {
    let Some(file) = args.first() else { usage() };
    let mut save = None;
    let mut lines = Vec::new();
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        if a == "--save" {
            save = Some(it.next().unwrap_or_else(|| usage()).clone());
        } else {
            lines.push(a.clone());
        }
    }
    let mut wb = open_or_exit(file);
    let mut failed = false;
    for l in &lines {
        failed |= wb.run(l, None).is_err();
    }
    if let Some(out) = save {
        failed |= wb.save_as(std::path::Path::new(&out)).is_err();
    }
    print_log(&wb, 0);
    let changed = wb.changed();
    if !changed.is_empty() {
        let list: Vec<String> = changed.iter().map(|h| format!("{h:X}")).collect();
        println!("# changed records: {}", list.join(" "));
    }
    if failed {
        exit(1);
    }
}

fn related(args: &[String]) {
    let (Some(file), Some(h)) = (args.first(), args.get(1).and_then(|s| parse_handle(s))) else {
        usage()
    };
    let depth = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(1);
    let mut wb = open_or_exit(file);
    wb.filemap = filemap::load(file).ok();
    for r in wb.related(h, depth) {
        let rel = match r.relation {
            Relation::Selected => "selected".to_string(),
            Relation::RefBy { from, depth } => format!("referenced by {from:X} (depth {depth})"),
            Relation::RefersTo { to } => format!("refers to {to:X}"),
        };
        let b = hex::block(&wb, r.handle, &rel);
        println!(
            "[{:X}] {:<28} {:>6} bytes {:<9} {:>3} fields  {rel}",
            r.handle,
            b.class,
            b.bytes.len(),
            b.source.label(),
            b.fields.len()
        );
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn svg_cmd(args: &[String]) {
    let (Some(out), Some(file)) = (args.first(), args.get(1)) else {
        usage()
    };
    let mut layers = view::Layers::default();
    let mut cam = view::Camera::new();
    let (mut w, mut h) = (1200.0, 800.0);
    let mut it = args[2..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--select" => layers.selected = it.next().and_then(|s| parse_handle(s)),
            "--3d" => cam.mode = view::Mode::Orbit,
            "--z-scale" => layers.z_scale = it.next().and_then(|s| s.parse().ok()).unwrap_or(1.0),
            "--no-proxy" => layers.proxy = false,
            "--no-decoded" => layers.decoded = false,
            "--placeholders" => layers.placeholders = true,
            "--no-context" => layers.context = false,
            "--light" => layers.dark = false,
            "--size" => {
                let v = it.next().unwrap_or_else(|| usage());
                let (a, b) = v.split_once('x').unwrap_or_else(|| usage());
                w = a.parse().unwrap_or_else(|_| usage());
                h = b.parse().unwrap_or_else(|_| usage());
            }
            _ => usage(),
        }
    }
    let wb = open_or_exit(file);
    cam.viewport = [0.0, 0.0, w, h];
    cam.fit(view::bounds(&wb.scene, &layers));
    let mut svg = svg::Svg::new(w, h, view::Palette::new(layers.dark).bg);
    view::draw(&wb.scene, &layers, &cam, &mut svg);
    if let Err(e) = std::fs::write(out, svg.finish()) {
        eprintln!("failed to write {out}: {e}");
        exit(1);
    }
}

/// The drawn objects (the viewer's `--dump`).
fn objects(path: &str) {
    let mut out = String::new();
    use scene::{opcode_name, station_text, Decoded};
    let s = &open_or_exit(path).scene;
    outln!(out, "{}", s.path);
    if let Some(r) = &s.failsafe {
        outln!(out, "  read in failsafe mode: {r}");
    }
    outln!(
        out,
        "  {} custom object(s), {} plain AutoCAD primitive(s)",
        s.objects.len(),
        s.context.len()
    );
    if !s.context_skipped.is_empty() {
        outln!(out, "  plain entities not drawn: {:?}", s.context_skipped);
    }
    for o in &s.objects {
        let dec = match &o.decoded {
            Decoded::None => "-".to_string(),
            Decoded::Failed(e) => format!("decode failed: {e}"),
            Decoded::Alignment(a) => format!(
                "alignment, {} tangent(s) {} arc(s), L={:.3}, sta {}..{}",
                a.tangents,
                a.arcs,
                a.length,
                station_text(a.start_station),
                station_text(a.start_station + a.length)
            ),
            Decoded::Profile(p) => format!(
                "profile over {:X}, {} entities, sta {}..{}, elev {:.3}..{:.3}, {} points",
                p.alignment,
                p.entities,
                station_text(p.stations.0),
                station_text(p.stations.1),
                p.elevations.0,
                p.elevations.1,
                p.prims.first().map_or(0, |l| match l {
                    scene::Prim::Line { pts, .. } => pts.len(),
                    _ => 0,
                })
            ),
        };
        let pg = match &o.proxy {
            None => "no proxy graphics".to_string(),
            Some(g) => {
                let skipped: Vec<String> = g
                    .skipped
                    .iter()
                    .map(|(op, n)| format!("{}×{n}", opcode_name(*op)))
                    .collect();
                format!(
                    "proxy: {} chunk(s), {} drawn{}{}",
                    g.chunk_count,
                    g.prims.len(),
                    if g.placeholder { ", placeholder" } else { "" },
                    if skipped.is_empty() {
                        String::new()
                    } else {
                        format!(", skipped [{}]", skipped.join(" "))
                    }
                )
            }
        };
        outln!(
            out,
            "  [{:X}] {} {:?}\n      {dec}\n      {pg}",
            o.handle,
            o.class,
            o.name.as_deref().unwrap_or("")
        );
    }
    // Ignore a closed pipe (`| head`).
    let _ = std::io::Write::write_all(&mut std::io::stdout(), out.as_bytes());
}

fn load_map(p: &str) -> filemap::Loaded {
    filemap::load(p).unwrap_or_else(|e| {
        eprintln!("failed to read {p}: {e}");
        exit(1);
    })
}

/// Sections and records of the file (hexview's `--dump`).
fn file_dump(path: &str) {
    let mut out = String::new();
    let l = load_map(path);
    outln!(out, "{} [{}]", l.path, l.version);
    for w in &l.warnings {
        outln!(out, "warning: {w}");
    }
    for p in &l.panes {
        let covered: usize = p.spans.iter().map(|s| s.len).sum();
        let bad_crc = p.spans.iter().filter(|s| !s.crc_ok).count();
        outln!(
            out,
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
            outln!(
                out,
                "{:>8X} {:>6} {:>8X} {:<9?} {}",
                s.start,
                s.len,
                s.handle,
                s.origin,
                s.label()
            );
        }
    }
    // Ignore a closed pipe (`| head`).
    let _ = std::io::Write::write_all(&mut std::io::stdout(), out.as_bytes());
}

/// Record-by-record comparison (hexview's `--diff`).
fn diff(args: &[String]) {
    let (Some(a), Some(b)) = (args.first(), args.get(1)) else {
        usage()
    };
    let show = flag(args, "--show").unwrap_or_else(|| "civil,opaque,unmapped".into());
    let filter = flag(args, "--filter").unwrap_or_default();
    let (la, lb) = (load_map(a), load_map(b));
    let c = compare::compare(&la, &lb);
    let want: Vec<&str> = show.split(',').map(str::trim).collect();
    let show = |o: filemap::Origin| {
        let name = match o {
            filemap::Origin::Acad => "acad",
            filemap::Origin::Civil => "civil",
            filemap::Origin::Opaque => "opaque",
            filemap::Origin::Unmapped => "unmapped",
        };
        want.contains(&name) || want.contains(&"all")
    };
    let text = compare::report(&c, &la, &lb, &show, &filter);
    // Ignore a closed pipe (`| head`).
    let _ = std::io::Write::write_all(&mut std::io::stdout(), text.as_bytes());
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--script") => script(&args[1..]),
        Some("--related") => related(&args[1..]),
        Some("--svg") => svg_cmd(&args[1..]),
        Some("--objects") => objects(args.get(1).unwrap_or_else(|| usage())),
        Some("--dump") => file_dump(args.get(1).unwrap_or_else(|| usage())),
        Some("--diff") => diff(&args[1..]),
        Some("-h" | "--help") => usage(),
        Some(s) if s.starts_with("--") && s != "--goto" && s != "--compare" => usage(),
        _ => {
            let mut file = None;
            let mut goto = None;
            let mut compare = None;
            let mut it = args.iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--goto" => {
                        goto = Some(
                            it.next()
                                .and_then(|h| parse_handle(h))
                                .unwrap_or_else(|| usage()),
                        )
                    }
                    "--compare" => compare = Some(it.next().unwrap_or_else(|| usage()).clone()),
                    _ => file = Some(a.clone()),
                }
            }
            if let Err(e) = app::run(file, goto, compare) {
                eprintln!("workbench failed to start: {e}");
                exit(1);
            }
        }
    }
}
