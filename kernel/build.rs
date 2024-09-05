use std::env;

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo::rerun-if-changed={dir}/src/init.S");
    println!("cargo::rerun-if-changed={dir}/kernel.ld");

    cc::Build::new()
        .file(format!("{dir}/src/init.S"))
        .target("riscv64-unkown-none-elf")
        .compile("init");

    println!("cargo::rustc-link-arg=-T{dir}/kernel.ld")
}
