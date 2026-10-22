//! Change the alignment style of a Civil 3D alignment and save the drawing.
//!
//! Usage:
//!   cargo run --release --example set_style -- in.dwg out.dwg <alignment> <style>
//!   cargo run --release --example set_style -- in.dwg            (list only)
//!
//! `<alignment>` is the alignment name or hex handle, `<style>` the style
//! name or hex handle. Only the style reference in the alignment's handle
//! stream changes (S18 vs S19); the style handle must have the same byte
//! length as the current one.

use civil3drust::aecc_alignment_refs::{alignment_refs, alignment_styles, set_alignment_style};
use civil3drust::raw;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn hex(s: &str) -> Option<u64> {
    u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()
}

fn list(doc: &acadrust::document::CadDocument) {
    let styles = alignment_styles(doc);
    let name_of = |h: u64| {
        styles
            .iter()
            .find(|s| s.0 == h)
            .map(|s| s.1.clone())
            .unwrap_or_else(|| "?".into())
    };
    println!("alignment styles:");
    for (h, n) in &styles {
        println!("  [{h:X}] {n}");
    }
    println!("alignments:");
    for rec in raw::records(doc, "AECC_ALIGNMENT") {
        let name = raw::object_name(&rec.data).unwrap_or_default();
        match alignment_refs(&rec) {
            Some(r) => println!(
                "  [{:X}] {name}: style [{:X}] {}",
                rec.handle,
                r.style.handle,
                name_of(r.style.handle)
            ),
            None => println!("  [{:X}] {name}: unknown handle-stream layout", rec.handle),
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 2 && a.len() != 5 {
        eprintln!("usage: set_style <in.dwg> [<out.dwg> <alignment> <style>]");
        std::process::exit(2);
    }
    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        eprintln!(
            "WARNING: acadrust read this file in failsafe mode; records it could not parse\n\
             will be MISSING from the output. ({r})"
        );
    }
    if a.len() == 2 {
        list(&opened.doc);
        return;
    }
    let align = raw::records(&opened.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| {
            raw::object_name(&r.data).as_deref() == Some(a[3].as_str())
                || Some(r.handle) == hex(&a[3])
        })
        .map(|r| r.handle)
        .unwrap_or_else(|| fail(format!("no alignment '{}'", a[3])));
    let style = alignment_styles(&opened.doc)
        .into_iter()
        .find(|(h, n)| n == &a[4] || Some(*h) == hex(&a[4]))
        .map(|s| s.0)
        .unwrap_or_else(|| fail(format!("no alignment style '{}'", a[4])));
    let old = set_alignment_style(&mut opened.doc, align, style).unwrap_or_else(|e| fail(e));
    println!("[{align:X}] style [{old:X}] -> [{style:X}]");
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    let reread =
        raw::open(&a[2]).unwrap_or_else(|e| fail(format!("written file does not read back: {e}")));
    println!("verify {}:", a[2]);
    list(&reread.doc);
}
