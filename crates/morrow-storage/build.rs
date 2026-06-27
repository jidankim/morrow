use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let candidates = [
        "/nix/store/nvdb98yjapy436iql7hqfpf1xkabwj0r-libiconv-1.19/lib",
        "/nix/store/bb4k2xb34m4z8sxjbx1np725yj57fy4h-libiconv-1.18/lib",
        "/opt/homebrew/lib",
        "/usr/local/lib",
    ];
    for candidate in candidates {
        if Path::new(candidate).join("libiconv.dylib").exists() {
            println!("cargo:rustc-link-search=native={candidate}");
            break;
        }
    }
}
