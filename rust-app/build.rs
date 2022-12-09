fn main() {
    println!("cargo:rerun-if-changed=script.ld");
    let profile = std::env::var("PROFILE").unwrap();
    let debug_print = std::env::var("CARGO_FEATURE_SPECULOS").is_ok();
    let extra_debug_print = std::env::var("CARGO_FEATURE_EXTRA_DEBUG").is_ok();
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let reloc_size = match (
        profile.as_str(),
        debug_print,
        extra_debug_print,
        target.as_str(),
    ) {
        ("release", false, false, "nanos") => 1872,
        ("release", false, false, "nanosplus") => 1912,
        (_, _, true, _) => 1024 * 10,
        _ => 1024 * 7,
    };
    println!("cargo:rustc-link-arg=--defsym=_reloc_size={reloc_size}");
}
