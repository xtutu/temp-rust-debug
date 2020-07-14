fn main() {
    if cfg!(all(feature = "builtin-lua", feature = "system-lua")) {
        panic!("cannot enable both builtin-lua and system-lua features when building rlua");
    }

    #[cfg(feature = "builtin-lua")]
    {
        use std::env;

        let target_os = env::var("CARGO_CFG_TARGET_OS");
        let target_family = env::var("CARGO_CFG_TARGET_FAMILY");

        let mut config = cc::Build::new();
        let mut config_binding = cc::Build::new();
        config.cpp(config.get_compiler().is_like_msvc());
        if target_os == Ok("linux".to_string()) {
            config.define("LUA_USE_LINUX", None);
        } else if target_os == Ok("macos".to_string()) {
            config.define("LUA_USE_MACOSX", None);
        } else if target_family == Ok("unix".to_string()) {
            config.define("LUA_USE_POSIX", None);
        } else if target_family == Ok("windows".to_string()) {
            config.define("LUA_USE_WINDOWS", None);
            config.define("LUA_BUILD_AS_DLL", None);
        }

        if cfg!(debug_assertions) {
            config.define("LUA_USE_APICHECK", None);
        }

        config_binding.define("RAPIDJSON_PARSE_DEFAULT_FLAGS", "kParseCommentsFlag");


        config.define("LUA_LIB", None);
        config
            .include("lua")
           // .include("lua-socket")
            .file("lua/lapi.c")
            .file("lua/lauxlib.c")
            .file("lua/lbaselib.c")
            .file("lua/lbitlib.c")
            .file("lua/lcode.c")
            .file("lua/lcorolib.c")
            .file("lua/lctype.c")
            .file("lua/ldblib.c")
            .file("lua/ldebug.c")
            .file("lua/ldo.c")
            .file("lua/ldump.c")
            .file("lua/lfunc.c")
            .file("lua/lgc.c")
            .file("lua/linit.c")
            .file("lua/liolib.c")
            .file("lua/llex.c")
            .file("lua/lmathlib.c")
            .file("lua/lmem.c")
            .file("lua/loadlib.c")
            .file("lua/lobject.c")
            .file("lua/lopcodes.c")
            .file("lua/loslib.c")
            .file("lua/lparser.c")
            .file("lua/lstate.c")
            .file("lua/lstring.c")
            .file("lua/lstrlib.c")
            .file("lua/ltable.c")
            .file("lua/ltablib.c")
            .file("lua/ltm.c")
            .file("lua/lundump.c")
            .file("lua/lutf8lib.c")
            .file("lua/lvm.c")
            .file("lua/lzio.c")
            .file("lua/pb.c")
            .file("lua/i64lib.c")

            // .file("lua-socket/src/auxiliar.c")
            // .file("lua-socket/src/buffer.c")
            // .file("lua-socket/src/compat.c")
            // .file("lua-socket/src/except.c")
            // .file("lua-socket/src/inet.c")
            //
            // .file("lua-socket/src/io.c")
            // .file("lua-socket/src/luasocket.c")
            // .file("lua-socket/src/mime.c")
            // .file("lua-socket/src/options.c")
            // .file("lua-socket/src/select.c")
            //
            // // .file("lua-socket/src/serial.c")
            // .file("lua-socket/src/tcp.c")
            // .file("lua-socket/src/timeout.c")
            // .file("lua-socket/src/udp.c")
            // .file("lua-socket/src/unix.c")
            //
            // // .file("lua-socket/src/unixdgram.c")
            // // .file("lua-socket/src/unixstream.c")
            // // .file("lua-socket/src/usocket.c")
            // .file("lua-socket/src/wsocket.c")

            .compile("lua5.3");

        config_binding
            .include("lua")
            .include("lua-rapidjson/include")
            .file("lua-rapidjson/source/rapidjson.cpp")
            .cpp(true)
            .compile("lua5.3-binding");
    }

    #[cfg(feature = "system-lua")]
    {
        pkg_config::Config::new()
            .atleast_version("5.3")
            .probe("lua")
            .unwrap();
    }
}
