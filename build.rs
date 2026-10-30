// Embeds assets/icon.ico in the Windows executable (Explorer, taskbar, shortcuts).
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "MACH5 Control");
        res.set("FileDescription", "MACH5 Control");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the icon: {}", e);
        }
    }
}
