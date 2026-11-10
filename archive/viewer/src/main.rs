//! civil3drust viewer: a 2D/3D look at the Civil 3D objects civil3drust can
//! read from a DWG, with the proxy graphics Civil 3D stored for them.
//!
//! ```text
//! civil3drust-viewer [file.dwg]                    open the window
//! civil3drust-viewer --dump file.dwg               list objects and exit
//! civil3drust-viewer --svg out.svg [options] file.dwg
//!     --3d              orbit view instead of plan
//!     --no-proxy        hide proxy graphics
//!     --no-decoded      hide decoded geometry
//!     --placeholders    include Civil 3D's "not drawn yet" placeholders
//!     --no-context      hide plain AutoCAD entities
//!     --light           light background
//!     --size WxH        image size (default 1200x800)
//! ```

mod app;
mod scene;
mod svg;
mod view;

use scene::{opcode_name, Decoded, Scene};
use view::{Camera, Layers, Mode};

fn usage() -> ! {
    eprintln!(
        "usage: civil3drust-viewer [file.dwg]\n       \
         civil3drust-viewer --dump file.dwg\n       \
         civil3drust-viewer --svg out.svg [--3d] [--no-proxy] [--no-decoded] \
         [--placeholders] [--no-context] [--light] [--size WxH] file.dwg"
    );
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--dump") => {
            let Some(path) = args.get(1) else { usage() };
            dump(&load_or_exit(path));
        }
        Some("--svg") => svg_cmd(&args[1..]),
        Some("-h" | "--help") => usage(),
        _ => {
            if let Err(e) = app::run(args.first().cloned()) {
                eprintln!("viewer failed to start: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn load_or_exit(path: &str) -> Scene {
    Scene::load(path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    })
}

fn dump(s: &Scene) {
    println!("{}", s.path);
    if let Some(r) = &s.failsafe {
        println!("  read in failsafe mode: {r}");
    }
    println!(
        "  {} custom object(s), {} plain AutoCAD primitive(s)",
        s.objects.len(),
        s.context.len()
    );
    if !s.context_skipped.is_empty() {
        println!("  plain entities not drawn: {:?}", s.context_skipped);
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
                scene::station_text(a.start_station),
                scene::station_text(a.start_station + a.length)
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
        println!(
            "  [{:X}] {} {:?}\n      {dec}\n      {pg}",
            o.handle,
            o.class,
            o.name.as_deref().unwrap_or("")
        );
    }
}

fn svg_cmd(args: &[String]) {
    let mut out = None;
    let mut file = None;
    let mut layers = Layers::default();
    let mut cam = Camera::new();
    let (mut w, mut h) = (1200.0, 800.0);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--3d" => cam.mode = Mode::Orbit,
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
            s if out.is_none() => out = Some(s.to_string()),
            s => file = Some(s.to_string()),
        }
    }
    let (Some(out), Some(file)) = (out, file) else {
        usage()
    };
    let scene = load_or_exit(&file);
    cam.viewport = [0.0, 0.0, w, h];
    cam.fit(view::bounds(&scene, &layers));
    let mut svg = svg::Svg::new(w, h, view::Palette::new(layers.dark).bg);
    view::draw(&scene, &layers, &cam, &mut svg);
    if let Err(e) = std::fs::write(&out, svg.finish()) {
        eprintln!("failed to write {out}: {e}");
        std::process::exit(1);
    }
}
