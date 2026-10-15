fn main() {
    // tauri-build builds icons/icon.ico into the .exe but doesn't watch it, so a new icon kept the old one (R57).
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
