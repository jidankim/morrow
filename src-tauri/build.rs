fn main() {
    #[cfg(target_os = "macos")]
    {
        compile_eventkit_native_bridge();
    }

    tauri_build::build();
}

#[cfg(target_os = "macos")]
fn compile_eventkit_native_bridge() {
    const NATIVE_BRIDGE_SOURCES: [&str; 3] = [
        "src/native_bridge/messages_attributed_body.m",
        "src/native_bridge/eventkit_cleanup.m",
        "src/native_bridge/eventkit_proposal.m",
    ];

    let mut build = cc::Build::new();
    for source in NATIVE_BRIDGE_SOURCES {
        println!("cargo:rerun-if-changed={source}");
        build.file(source);
    }
    build.flag("-fobjc-arc");
    if !build.get_compiler().is_like_clang() {
        build.compiler("clang");
    }
    build.compile("morrow_eventkit_native_bridge");

    println!("cargo:rustc-link-lib=framework=EventKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
