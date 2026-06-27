fn main() {
    #[cfg(target_os = "macos")]
    {
        compile_eventkit_cleanup();
    }

    tauri_build::build();
}

#[cfg(target_os = "macos")]
fn compile_eventkit_cleanup() {
    let mut build = cc::Build::new();
    build
        .file("src/native_bridge/eventkit_cleanup.m")
        .flag("-fobjc-arc");
    if !build.get_compiler().is_like_clang() {
        build.compiler("clang");
    }
    build.compile("morrow_eventkit_cleanup");

    println!("cargo:rustc-link-lib=framework=EventKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
