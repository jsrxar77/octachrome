//! Compiles the vendored oscillator DSP (C++11, MIT) into a static library.
fn main() {
    let mut b = cc::Build::new();
    b.cpp(true)
        .std("c++11")
        .include("vendor")
        .define("TEST", None) // desktop configuration of the vendored code
        // user_data.h uses printf in desktop mode without including <cstdio>
        .flag("-include")
        .flag("cstdio")
        .warnings(false)
        .opt_level(2)
        .file("src/shim.cc");
    for entry in walk("vendor") {
        if entry.extension().is_some_and(|e| e == "cc") {
            b.file(&entry);
        }
    }
    b.compile("op8_macro_dsp");
    println!("cargo:rerun-if-changed=src/shim.cc");
    println!("cargo:rerun-if-changed=vendor");
}

fn walk(dir: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("vendor dir").flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}
