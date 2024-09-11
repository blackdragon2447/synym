use std::env;

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo::rerun-if-changed={dir}/kernel.ld");

    println!("cargo::rustc-link-arg=-T{dir}/kernel.ld")
}
