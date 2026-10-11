/// The Visual C++ runtime DLLs a packaged fettle.exe loads from its own folder (packaging/runtime.mjs
/// ships them and checks that every one it ships is named here).
const APP_LOCAL_RUNTIME: [&str; 3] = ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"];

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
        // Packaged builds (FETTLE_APP_LOCAL_RUNTIME=1, set by packaging/build.mjs) ship the Visual C++
        // runtime beside fettle.exe, and this manifest makes Windows load it from there. Other builds
        // use the PC's own copy: with the manifest and no DLLs beside it, the exe wouldn't start.
        println!("cargo:rerun-if-env-changed=FETTLE_APP_LOCAL_RUNTIME");
        if std::env::var("FETTLE_APP_LOCAL_RUNTIME").is_ok_and(|v| v == "1") {
            let files: String = APP_LOCAL_RUNTIME.iter().map(|f| format!("  <file name=\"{f}\"/>\n")).collect();
            res.set_manifest(&format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
                 <assembly xmlns=\"urn:schemas-microsoft-com:asm.v1\" manifestVersion=\"1.0\">\n\
                 \x20 <assemblyIdentity type=\"win32\" name=\"Fettle\" version=\"1.0.0.0\" processorArchitecture=\"amd64\"/>\n\
                 {files}</assembly>\n"
            ));
        }
        res.compile().expect("the exe's icon and version details");
        // Every DLL fettle.exe imports loads from System32 only, never from the folder it sits in, so a
        // DLL left beside it (in Downloads, say) can't load into it, even as the Deep check's
        // administrator helper. The runtime named in the manifest above is the one exception.
        println!("cargo:rustc-link-arg-bin=fettle=/DEPENDENTLOADFLAG:0x800");
        // That flag covers only the DLLs fettle.exe imports, not what they import in turn: sspicli.dll
        // (from secur32), netutils.dll and samcli.dll (from netapi32) and glu32.dll (from opengl32)
        // would still be looked for beside it first. These three load when first used instead, after
        // main has limited every load to System32 (SetDefaultDllDirectories), their own included.
        // packaging/runtime.mjs plants every DLL in the chain beside a packaged copy to check.
        for dll in ["secur32.dll", "netapi32.dll", "opengl32.dll"] {
            println!("cargo:rustc-link-arg-bin=fettle=/DELAYLOAD:{dll}");
        }
        println!("cargo:rustc-link-arg-bin=fettle=delayimp.lib");
    }
}
