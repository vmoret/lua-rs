use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS");
    let target_family = env::var("CARGO_CFG_TARGET_FAMILY");

    let mut config = cc::Build::new();

    if target_os == Ok("linux".to_string()) {
        config.define("LUA_USE_LINUX", None);
    } else if target_os == Ok("macos".to_string()) {
        config.define("LUA_USE_MACOSX", None);
    } else if target_family == Ok("unix".to_string()) {
        config.define("LUA_USE_POSIX", None);
    } else if target_family == Ok("windows".to_string()) {
        config.define("LUA_USE_WINDOWS", None);
    }

    if cfg!(debug_assertions) {
        config.define("LUA_USE_APICHECK", None);
    }

    config
        .include("lua")
        .file("src/ffi/lapi.c")
        .file("src/ffi/lauxlib.c")
        .file("src/ffi/lbaselib.c")
        .file("src/ffi/lcode.c")
        .file("src/ffi/lcorolib.c")
        .file("src/ffi/lctype.c")
        .file("src/ffi/ldblib.c")
        .file("src/ffi/ldebug.c")
        .file("src/ffi/ldo.c")
        .file("src/ffi/ldump.c")
        .file("src/ffi/lfunc.c")
        .file("src/ffi/lgc.c")
        .file("src/ffi/linit.c")
        .file("src/ffi/liolib.c")
        .file("src/ffi/llex.c")
        .file("src/ffi/lmathlib.c")
        .file("src/ffi/lmem.c")
        .file("src/ffi/loadlib.c")
        .file("src/ffi/lobject.c")
        .file("src/ffi/lopcodes.c")
        .file("src/ffi/loslib.c")
        .file("src/ffi/lparser.c")
        .file("src/ffi/lstate.c")
        .file("src/ffi/lstring.c")
        .file("src/ffi/lstrlib.c")
        .file("src/ffi/ltable.c")
        .file("src/ffi/ltablib.c")
        .file("src/ffi/ltm.c")
        .file("src/ffi/lundump.c")
        .file("src/ffi/lutf8lib.c")
        .file("src/ffi/lvm.c")
        .file("src/ffi/lzio.c")
        .compile("liblua5.4.a");
}
