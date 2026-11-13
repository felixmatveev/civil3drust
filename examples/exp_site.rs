//! Site write experiments (docs/site-object.md). The writers proved here
//! now live in `civil3drust::aecc_site_edit` (also as `c3dr site_*`
//! commands); this example keeps the experiments reproducible.
//!
//! Usage: cargo run --release --example exp_site -- <command> in.dwg out.dwg args...
//!
//!   rename   <site hex> <new name> [description]   name (and description) strings
//!   create   <name>                                 a new empty site, cloned from the first site
//!   delete   <site hex>                             an empty site, its container and list entry
//!   move-in  <alignment hex> <site hex>             a siteless alignment that crosses nothing
//!   move-out <alignment hex>                        back to siteless (an emptied companion is refilled)
//!   pool     <id hex> ...                           one empty site per built-in name ID (MIST22, S352)
//!   show                                            print sites, containers and companions (no out.dwg)

use civil3drust::aecc_site_edit as edit;
use civil3drust::raw;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap_or_else(|_| fail(format!("not a handle: {s}")))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        fail(
            "usage: exp_site <rename|create|delete|move-in|move-out|pool|show> in.dwg [out.dwg args...]",
        );
    }
    let mut o = raw::open(&a[2]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &o.failsafe_reason {
        fail(format!("read in failsafe mode ({r})"));
    }
    if a[1] == "show" {
        return print!("{}", edit::describe(&o.doc).unwrap_or_else(|e| fail(e)));
    }
    let args = a.get(4..).unwrap_or_default();
    let need = |n: usize| {
        if args.len() < n {
            fail(format!("{}: {n} argument(s) expected", a[1]));
        }
    };
    let doc = &mut o.doc;
    let r = match a[1].as_str() {
        "rename" => {
            need(2);
            edit::rename_site(
                doc,
                hex(&args[0]),
                Some(&args[1]),
                args.get(2).map(String::as_str),
            )
        }
        "create" => {
            need(1);
            edit::create_site(doc, &args[0]).map(|c| {
                println!(
                    "site {:X}, container {:X}, key {:08X}",
                    c.site, c.container, c.key
                )
            })
        }
        "delete" => {
            need(1);
            edit::delete_site(doc, hex(&args[0]))
        }
        "move-in" => {
            need(2);
            edit::move_into_site(doc, hex(&args[0]), hex(&args[1])).map(|m| {
                println!(
                    "{:X} → site {:X}; companion {:X} emptied",
                    m.alignment, m.site, m.companion
                )
            })
        }
        "move-out" => {
            need(1);
            edit::move_out_of_site(doc, hex(&args[0])).map(|m| {
                println!(
                    "{:X} → siteless (companion {:X} refilled)",
                    m.alignment, m.companion
                )
            })
        }
        "pool" => {
            need(1);
            args.iter().try_for_each(|id| {
                let key = u32::try_from(hex(id)).unwrap_or_else(|_| fail("ID above 32 bits"));
                edit::create_site_with_builtin_name(doc, key, &format!("id {key:08X}")).map(|_| ())
            })
        }
        c => fail(format!("unknown command {c}")),
    };
    r.unwrap_or_else(|e| fail(e));
    acadrust::DwgWriter::write_to_file(&a[3], &o.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[3]);
}
