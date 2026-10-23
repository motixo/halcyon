fn main() {
    // Positive local-build identity. A copied binary is not a verified source
    // installation, and a package beside this build never owns this process.
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let profile = out.ancestors().nth(3).unwrap();
    println!(
        "cargo:rustc-env=HALCYON_BUILD_BINARY={}",
        profile.join("halcyon").display()
    );
    println!("cargo:rerun-if-changed=build.rs");
}
