fn main() {
    tauri_build::build();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" && target_os != "linux" {
        return;
    }

    // Ensure Cargo reruns this build script when native sources change.
    // Common (vendor-neutral)
    println!("cargo:rerun-if-changed=native/common/Pal.cpp");
    println!("cargo:rerun-if-changed=native/common/Pal.h");
    println!("cargo:rerun-if-changed=native/common/runtime_dirs.cpp");
    println!("cargo:rerun-if-changed=native/common/runtime_dirs.h");
    println!("cargo:rerun-if-changed=native/common/bridge_util.cpp");
    println!("cargo:rerun-if-changed=native/common/bridge_util.h");
    // J-Link backend
    println!("cargo:rerun-if-changed=native/jlink/jlink_bridge.h");
    println!("cargo:rerun-if-changed=native/jlink/jlink_bridge_api.cpp");
    println!("cargo:rerun-if-changed=native/jlink/bridge_state.h");
    println!("cargo:rerun-if-changed=native/jlink/bridge_state.cpp");
    println!("cargo:rerun-if-changed=native/jlink/JLinkARMDLL_Wrapper.cpp");
    println!("cargo:rerun-if-changed=native/jlink/JLinkARMDLL_Wrapper.h");
    println!("cargo:rerun-if-changed=native/jlink/commander_exec.h");
    println!("cargo:rerun-if-changed=native/jlink/commander_exec.cpp");
    println!("cargo:rerun-if-changed=native/jlink/JLinkARMDLL.h");
    println!("cargo:rerun-if-changed=native/jlink/JLINKARM_Const.h");
    println!("cargo:rerun-if-changed=native/jlink/TYPES.h");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        // Single root: includes use `common/…` and `jlink/…` prefixes.
        .include("native")
        .file("native/common/Pal.cpp")
        .file("native/common/runtime_dirs.cpp")
        .file("native/common/bridge_util.cpp")
        .file("native/jlink/bridge_state.cpp")
        .file("native/jlink/JLinkARMDLL_Wrapper.cpp")
        .file("native/jlink/commander_exec.cpp")
        .file("native/jlink/jlink_bridge_api.cpp");

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
