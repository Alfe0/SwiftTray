fn main() {
    println!("cargo:rerun-if-changed=assets/SwiftTray.ico");
    println!("cargo:rerun-if-changed=assets/SwiftTray-tray.ico");
    if cfg!(target_os = "windows") {
        let mut resource = winres::WindowsResource::new();
        resource
            .set_icon("assets/SwiftTray.ico")
            // ID 1 stays the default Windows application icon.
            // ID 2 is loaded explicitly by the notification area only.
            .set_icon_with_id("assets/SwiftTray-tray.ico", "2")
            .set("FileDescription", "SwiftTray")
            .set("ProductName", "SwiftTray")
            .set("FileVersion", env!("CARGO_PKG_VERSION"))
            .set("ProductVersion", env!("CARGO_PKG_VERSION"))
            .set("OriginalFilename", "SwiftTray.exe")
            .set("InternalName", "SwiftTray.exe");
        resource.compile().expect("Windows resource compilation failed");
    }
}
