mod lua_test;
mod lua_log;
mod lua_file;
mod lua_time;
mod lua_xt;
mod lua_network;
mod lua_http;

pub use lua_test::try_lua;
pub use lua_test::try_lua3;
pub use log;
use std::collections::HashMap;
use lazy_static::lazy_static;
use rlua::{Function, Lua, MetaMethod, Result, UserData, UserDataMethods, Variadic, Error, MultiValue};
use std::process::Command;
use std::path::PathBuf;
use std::path;
use xkit;
use std::sync::{Mutex, RwLock};
use crate::conf;
use xkit::file_util::format_path;

pub fn new_lua_state(uuid_string: String) -> Lua {
    let lua = unsafe {
        Lua::new_with_debug()
    };

    lua.context(|context|{
        let cfg = conf::ins().read().unwrap();
        if cfg.use_dist {
            context.load("package.path = package.cpath .. \";./dist/?.lua\"").exec().unwrap();
        }else{
            context.load("package.path = package.cpath .. \";./code/?.lua\"").exec().unwrap();
        }
    });

    lua.context(|context| {
        let globals = context.globals();
        globals.set("__uuid__", uuid_string);

        lua_log::register(context);
        lua_file::register(context);
        lua_time::register(context);
        lua_xt::register(context);
        lua_network::register(context);
        lua_http::register(context)
    });

    lua
}

pub fn do_require_file(context: rlua::Context, mut module_name: String) {
    // module_name = module_name.replace("/", ".");
    module_name = module_name.replace("\\", ".");
    let result = context.load(&("require('".to_string() + module_name.as_str() + "')")).exec();
    if let Err(e) = result {
        log::error!("require lua:{:?} error {:?}", module_name,  e);
        return;
    }
}

lazy_static! {
    static ref CACHE_LUA_MAP : RwLock<HashMap::<String, Vec::<u8>>>= {
        let mut m = HashMap::new();
        RwLock::new(m)
    };
}

pub fn do_require_file_by_cache(context: rlua::Context, mut module_name: String) {
    // let CACHE_LUA_MAP: HashMap::<string, Vec::<u8>>= HashMap::new();
    let mut cache_lua_map_temp = CACHE_LUA_MAP.write().unwrap();
    let cfg = conf::ins().read().unwrap();

    let full_path = path::Path::new(&cfg.root_dir).join(get_root_dir().as_str()).join(module_name.clone() + ".lua");


    // log::info!("{:?}, {:?}, {:?}, full_path: {:?}",
    //     &cfg.root_dir,
    //     get_root_dir().as_str(),
    //     module_name.clone() + ".lua",
    //     full_path);
    // log::info!("full_path: {:?}", full_path.to_str().unwrap());

    module_name = module_name.replace("\\", ".");
    if !cache_lua_map_temp.contains_key(&module_name) {
        let result = xkit::file_util::load_file_byte(&full_path);
        if let Ok(data) = result {
            cache_lua_map_temp.insert(module_name.clone(), data);
        } else {
            log::error!("{:?} {:?}", &full_path, result.unwrap_err());
        }
    }

    let data = cache_lua_map_temp.get(&module_name).unwrap();

    let result = context.load(data).set_name(&module_name).expect("set name error").exec();
    if let Err(e) = result {
        log::error!("require lua:{:?} error {}", module_name,  e);
        return;
    }
}


// pub fn do_require_file_by_cache(context: rlua::Context, mut module_name: String) {
//     // let CACHE_LUA_MAP: HashMap::<string, Vec::<u8>>= HashMap::new();
//     let mut cache_lua_map_temp = CACHE_LUA_MAP.lock().unwrap();
//     let cfg = conf::ins().lock().unwrap();
//     let full_path = path::Path::new(&cfg.root_dir).join("code").join(module_name.clone() + ".lua");
//     // log::info!("full_path: {:?}", full_path);
//     // log::info!("full_path: {:?}", full_path.to_str().unwrap());
//
//     let temp_name = "temp_lua.o";
//     module_name = module_name.replace("\\", ".");
//     if !cache_lua_map_temp.contains_key(&module_name) {
//         let output = Command::new("luac")
//             .args(&["-o", temp_name, full_path.to_str().unwrap()])
//             .output()
//             .expect("failed to execute process");
//         // log::info!("{:?}", output);
//         if !output.stderr.is_empty(){
//             log::info!("{:?}", output);
//             panic!();
//         }
//         let data = xkit::file_util::load_file_byte(temp_name).unwrap();
//         cache_lua_map_temp.insert(module_name.clone(), data);
//     }
//
//     let data = cache_lua_map_temp.get(&module_name).unwrap();
//
//     let result = context.load(data).set_name(&module_name).expect("set name error").exec();
//     if let Err(e) = result {
//         log::error!("require lua:{:?} error {:?}", module_name,  e);
//         return;
//     }
// }


pub fn compile(source_path: String) {
    // let cfg = conf::ins().lock().unwrap();
    // let full_path = path::Path::new(&cfg.root_dir).join("code").join(module_name.clone() + ".lua");
    // let full_path = source_path;
    let source_path = format_path(source_path);
    let to_path = source_path.replace("code/", "dist/");
    // log::info!("{} => {}", &source_path, &to_path);

    if let Err(e) = xkit::file_util::try_create_dir(&to_path, true) {
        log::error!("{:?}", e);
        panic!()
    }

    let output = Command::new("luac")
        .args(&["-o", &to_path, source_path.as_str()])
        .output()
        .expect("failed to execute process");


    if !output.stderr.is_empty() {
        log::info!("{:?}", output);
        panic!();
    }
    // log::info!("{:?}", output);
}

// const dist_dir = "dist"
pub fn get_root_dir()->String{
    return if conf::ins().read().unwrap().use_dist {
        "dist".to_string()
    } else {
        "code".to_string()
    }
}
