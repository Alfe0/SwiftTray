fn main() {
    println!("cargo:rerun-if-changed=assets/SwiftTray.ico");
    if cfg!(target_os = "windows") {
        let mut resource = winres::WindowsResource::new();
        resource
            .set_icon("assets/SwiftTray.ico")
            .set("FileDescription", "SwiftTray")
            .set("ProductName", "SwiftTray")
            .set("FileVersion", env!("CARGO_PKG_VERSION"))
            .set("ProductVersion", env!("CARGO_PKG_VERSION"))
            .set("OriginalFilename", "SwiftTray.exe")
            .set("InternalName", "SwiftTray.exe");
        resource.compile().expect("Windows resource compilation failed");
    }
}
