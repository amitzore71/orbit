fn main() {
    slint_build::compile_with_config(
        "ui/app.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent".into()),
    )
    .expect("compile Slint interface");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/orbit.ico")
            .set("ProductName", "Orbit")
            .set("FileDescription", "Orbit · Focus timer")
            .set("LegalCopyright", "Copyright © 2026 Orbit contributors")
            .set_manifest_file("assets/app.manifest")
            .compile()
            .expect("compile Windows resources");
    }
}
