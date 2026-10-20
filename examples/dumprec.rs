//! Write the raw bytes of every record of one AECC class to files.
//!
//! Usage: cargo run --release --example dumprec -- file.dwg AECC_ALIGNMENT out_prefix
//! Produces `<out_prefix>_<HANDLE>.bin` per record.

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 4 {
        eprintln!("usage: dumprec <file.dwg> <AECC_CLASS> <out_prefix>");
        std::process::exit(2);
    }
    let opened = civil3drust::raw::open(&a[1]).unwrap_or_else(|e| {
        eprintln!("failed to read {}: {e}", a[1]);
        std::process::exit(1);
    });
    for r in civil3drust::raw::records(&opened.doc, &a[2]) {
        let path = format!("{}_{:X}.bin", a[3], r.handle);
        std::fs::write(&path, &r.data).expect("write");
        println!("{path}: {} bytes", r.data.len());
    }
}
