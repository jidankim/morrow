fn main() {
    #[cfg(target_os = "macos")]
    {
        compile_eventkit_native_bridge();
    }

    tauri_build::build();
}

#[cfg(target_os = "macos")]
fn compile_eventkit_native_bridge() {
    let mut build = cc::Build::new();
    build
        .file("src/native_bridge/eventkit_cleanup.m")
        .file("src/native_bridge/eventkit_proposal.m")
        .flag("-fobjc-arc");
    if !build.get_compiler().is_like_clang() {
        build.compiler("clang");
    }
    build.compile("morrow_eventkit_native_bridge");

    println!("cargo:rustc-link-lib=framework=EventKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
