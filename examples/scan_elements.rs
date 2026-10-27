//! Decode a raw `AECC_ALIGNMENT` record dump (scratch tool for records
//! rebuilt from hexview diffs).
use civil3drust::aecc_elements::{decode_elements, element_records};
use civil3drust::raw::RawRecord;
fn main() {
    let mut a = std::env::args().skip(1);
    let (p, hb): (String, usize) = (
        a.next().unwrap(),
        a.next().map_or(0, |s| s.parse().unwrap()),
    );
    let d = std::fs::read(&p).unwrap();
    for e in element_records(&d) {
        println!("  {e:?}");
    }
    let rec = RawRecord {
        handle: 0,
        class: "AECC_ALIGNMENT".into(),
        is_entity: true,
        data: d,
        handle_bits: hb,
    };
    match decode_elements(&rec) {
        Ok(x) => {
            println!(
                "start station {:?}, length {:.4}",
                x.stored_start_station,
                x.length()
            );
            for e in &x.elements {
                println!("  {:?}", e.segment);
            }
        }
        Err(e) => println!("decode failed: {e}"),
    }
}
