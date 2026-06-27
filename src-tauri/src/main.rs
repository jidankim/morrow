#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
embed_plist::embed_info_plist!("../Info.plist");

fn main() {
    if let Err(error) = morrow_lib::run() {
        eprintln!("failed to start Morrow: {error}");
        std::process::exit(1);
    }
}
