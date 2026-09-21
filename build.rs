fn main() {
    println!("cargo:rerun-if-changed=assets/windows.rc");
    println!("cargo:rerun-if-changed=assets/AppIcon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // 图标是发布必需资源；失败时停止构建，避免再次发布空白图标程序。
        embed_resource::compile(
            "assets/windows.rc",
            embed_resource::ParamsIncludeDirs(["assets"]),
        )
        .manifest_required()
        .expect("Windows 应用图标资源编译失败");
    }
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
