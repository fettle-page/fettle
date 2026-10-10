fn main() {
    // Debug builds (and so the tests) carry element names for Slint's testing API; release builds
    // stay lean. Screen readers don't need this: accessibility works in both.
    let debug = std::env::var("PROFILE").is_ok_and(|p| p == "debug");
    let config = slint_build::CompilerConfiguration::new().with_debug_info(debug);
    slint_build::compile_with_config("ui/app.slint", config).expect("the Slint UI compiles");

    // The exe's own icon and details, as Explorer, the taskbar and Windows' Apps list show them.
    // The version comes from Cargo.toml.
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        println!("cargo:rerun-if-changed=ui/img/fettle.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("ui/img/fettle.ico")
            .set("ProductName", "Fettle")
            .set("FileDescription", "Fettle")
            .set("CompanyName", "Fettle")
            .set("LegalCopyright", "MIT licence")
            .set("OriginalFilename", "fettle.exe");
        res.compile().expect("the exe's icon and version details");
    }
}
