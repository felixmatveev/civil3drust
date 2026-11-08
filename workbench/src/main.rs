//! civil3drust workbench: open a DWG, look at it, inspect objects as hex
//! and RON, edit them with commands, save a copy (D-073, D-074).
//!
//! ```text
//! civil3drust-workbench [file.dwg] [--goto <handle>]
//!                                                  open the window
//! civil3drust-workbench --script file.dwg [--save out.dwg] "command" ...
//!                                                  run commands headless, print the log
//! civil3drust-workbench --related file.dwg <handle> [depth]
//!                                                  list the records the hex pane shows
//! civil3drust-workbench --svg out.svg file.dwg [--select <handle>] [--light]
//!                                                  render the view to SVG
//! ```

mod app;
// Copied whole from hexview/ (its file-offset view and compare come next,
// docs/workbench.md); parts not used yet are kept for that.
#[allow(dead_code)]
mod filemap;
mod hex;
#[allow(dead_code)]
mod scene;
mod state;
mod svg;
mod view;

use civil3drust::commands::parse_handle;
use state::{LogKind, Relation, Workbench};
use std::process::exit;

fn usage() -> ! {
    eprintln!(
        "usage: civil3drust-workbench [file.dwg] [--goto <handle>]\n       \
         civil3drust-workbench --script file.dwg [--save out.dwg] \"command\" ...\n       \
         civil3drust-workbench --related file.dwg <handle> [depth]\n       \
         civil3drust-workbench --svg out.svg file.dwg [--select <handle>] [--light]"
    );
    exit(2);
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

fn svg_cmd(args: &[String]) {
    let (Some(out), Some(file)) = (args.first(), args.get(1)) else {
        usage()
    };
    let mut layers = view::Layers::default();
    let mut it = args[2..].iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--select" => layers.selected = it.next().and_then(|s| parse_handle(s)),
            "--light" => layers.dark = false,
            _ => usage(),
        }
    }
    let wb = open_or_exit(file);
    let (w, h) = (1200.0, 800.0);
    let mut cam = view::Camera::new();
    cam.viewport = [0.0, 0.0, w, h];
    cam.fit(view::bounds(&wb.scene, &layers));
    let mut svg = svg::Svg::new(w, h, view::Palette::new(layers.dark).bg);
    view::draw(&wb.scene, &layers, &cam, &mut svg);
    if let Err(e) = std::fs::write(out, svg.finish()) {
        eprintln!("failed to write {out}: {e}");
        exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--script") => script(&args[1..]),
        Some("--related") => related(&args[1..]),
        Some("--svg") => svg_cmd(&args[1..]),
        Some("-h" | "--help") => usage(),
        Some(s) if s.starts_with("--") && s != "--goto" => usage(),
        _ => {
            let mut file = None;
            let mut goto = None;
            let mut it = args.iter();
            while let Some(a) = it.next() {
                if a == "--goto" {
                    goto = Some(
                        it.next()
                            .and_then(|h| parse_handle(h))
                            .unwrap_or_else(|| usage()),
                    );
                } else {
                    file = Some(a.clone());
                }
            }
            if let Err(e) = app::run(file, goto) {
                eprintln!("workbench failed to start: {e}");
                exit(1);
            }
        }
    }
}
