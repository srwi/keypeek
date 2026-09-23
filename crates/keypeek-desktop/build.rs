fn main() {
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("../../resources/icon.ico")
            .compile()
            .expect("Failed to embed Windows resources.");
    }
}
