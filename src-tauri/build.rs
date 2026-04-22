fn main() {
    tauri_build::build();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" && target_os != "linux" {
        return;
    }

    // Ensure Cargo reruns this build script when native sources change.
    println!("cargo:rerun-if-changed=native/Pal.cpp");
    println!("cargo:rerun-if-changed=native/Pal.h");
    println!("cargo:rerun-if-changed=native/JLinkARMDLL_Wrapper.cpp");
    println!("cargo:rerun-if-changed=native/JLinkARMDLL_Wrapper.h");
    println!("cargo:rerun-if-changed=native/jlink_bridge.h");
    println!("cargo:rerun-if-changed=native/jlink_bridge_api.cpp");
    println!("cargo:rerun-if-changed=native/bridge_support.h");
    println!("cargo:rerun-if-changed=native/bridge_support.cpp");
    println!("cargo:rerun-if-changed=native/runtime_dirs.h");
    println!("cargo:rerun-if-changed=native/runtime_dirs.cpp");
    println!("cargo:rerun-if-changed=native/commander_exec.h");
    println!("cargo:rerun-if-changed=native/commander_exec.cpp");
    println!("cargo:rerun-if-changed=native/JLinkARMDLL.h");
    println!("cargo:rerun-if-changed=native/JLINKARM_Const.h");
    println!("cargo:rerun-if-changed=native/TYPES.h");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .include("native")
        .file("native/Pal.cpp")
        .file("native/JLinkARMDLL_Wrapper.cpp")
        .file("native/bridge_support.cpp")
        .file("native/runtime_dirs.cpp")
        .file("native/commander_exec.cpp")
        .file("native/jlink_bridge_api.cpp");

    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "msvc" {
        build.flag("/EHsc");
    }

    build.compile("jlink_arm_bridge");

    if target_os == "linux" {
        // Pal.cpp uses dlopen/dlsym; link libdl.
        println!("cargo:rustc-link-lib=dl");
    }
}
