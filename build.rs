fn main() {
    println!("cargo:rerun-if-changed=BUILD_NUMBER");
    println!(
        "cargo:rustc-env=APP_VERSION={}.{}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR")
    );
    println!(
        "cargo:rustc-env=APP_BUILD={}",
        std::fs::read_to_string("BUILD_NUMBER").unwrap().trim()
    );
    slint_build::compile("ui/app.slint").unwrap();
}
