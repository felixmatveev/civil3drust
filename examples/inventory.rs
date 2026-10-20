//! List the Civil 3D (AECC) classes in a DWG file.
//!
//! Usage: cargo run --example inventory -- path/to/drawing.dwg

use civil3drust::inventory::aecc_classes_in_file;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: inventory <file.dwg>");
        std::process::exit(2);
    };

    match aecc_classes_in_file(&path) {
        Ok(classes) if classes.is_empty() => println!("No AECC classes found in {path}"),
        Ok(classes) => {
            println!(
                "{:<40} {:<40} {:>6} {:>9}  entity",
                "DXF name", "C++ class", "num", "instances"
            );
            for c in classes {
                println!(
                    "{:<40} {:<40} {:>6} {:>9}  {}",
                    c.dxf_name, c.cpp_class_name, c.class_number, c.instance_count, c.is_entity
                );
            }
        }
        Err(e) => {
            eprintln!("failed to read {path}: {e}");
            std::process::exit(1);
        }
    }
}
