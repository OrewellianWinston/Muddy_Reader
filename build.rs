fn main() {
    println!("cargo:rerun-if-changed=assets/md-reader.ico");

    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("assets/md-reader.ico")
        .compile()
        .expect("failed to embed the MD Reader Windows icon");
}
