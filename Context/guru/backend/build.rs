//! Generates the `books!(...)` list from src/books (standard library only).
//! A file in src/books is a book module if a line starts with `pub fn catalogue`.
//! Order: src/books/ORDER.txt first (frozen 31 Oct 2026, never edit it: catalogue order is shown in the UI),
//! then every other book module in alphabetical order.
//! It also collects every `pub const ADMISSION...: Admission` in src/books/*.rs into books::ADMISSIONS (alphabetical by
//! file), the per-book instrument admission table read by instrument_policy.rs.
use std::{env, fs, path::Path};

fn main() {
    let dir = Path::new("src/books");
    println!("cargo:rerun-if-changed=src/books");
    let mut files: Vec<(String, String)> = fs::read_dir(dir).expect("read src/books").filter_map(|e| {
        let p = e.ok()?.path();
        let name = p.file_stem()?.to_str()?.to_string();
        if p.extension()? != "rs" || name == "mod" { return None; }
        Some((name, fs::read_to_string(&p).ok()?))
    }).collect();
    files.sort();
    let found: Vec<String> = files.iter().filter(|(_, text)| text.lines().any(|l| l.trim_start().starts_with("pub fn catalogue")))
        .map(|(name, _)| name.clone()).collect();
    let admissions: Vec<String> = files.iter().flat_map(|(name, text)| text.lines()
        .filter_map(|l| l.strip_prefix("pub const ADMISSION")?.split(':').next().map(|rest| format!("&{name}::ADMISSION{rest}")))
        .collect::<Vec<_>>()).collect();
    let order: Vec<&str> = include_str!("src/books/ORDER.txt").lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let mut list: Vec<String> = Vec::new();
    for m in &order {
        assert!(found.iter().any(|f| f == m), "src/books/ORDER.txt names `{m}`, but src/books/{m}.rs has no `pub fn catalogue`");
        assert!(!list.iter().any(|l| l == m), "src/books/ORDER.txt names `{m}` twice");
        list.push(m.to_string());
    }
    list.extend(found.into_iter().filter(|f| !order.contains(&f.as_str())));
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("books_list.rs");
    // A macro, not a `books!(...)` call: `mod` items expanded inside an include!d file would be looked up next to OUT_DIR.
    fs::write(out, format!("macro_rules! with_book_list {{ ($m:ident) => {{ $m!({}); }}; }}\n\
        pub const ADMISSIONS: &[&crate::instrument_policy::Admission] = &[{}];\n", list.join(", "), admissions.join(", "))).unwrap();
}
