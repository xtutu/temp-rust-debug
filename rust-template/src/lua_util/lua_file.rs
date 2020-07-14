use xkit::file_util;
use futures::io::Error;
use rlua::Table;
use bstr::{BString, BStr};
use bstr::{ByteSlice, ByteVec};
use std::path::Path;

pub fn register(context: rlua::Context) {
    let globals = context.globals();
    let lua_file_util_table = context.create_table().unwrap();

    {
        let load_file_byte = context.create_function::<String, BString, _>(|_context, file_path| -> Result<BString, _> {
            // file_util::load_file_byte(file_path).map_err(|_|{
            //     rlua::Error::UserDataBorrowMutError
            // })
            let ret = file_util::load_file_byte(file_path);
            if let Err(e)  = ret{
                log::error!("err {}", e);
                return Ok(BString::from(""));
            }
            Ok(BString::from(ret.unwrap()))

        }).expect("load_file error");


        // let load_file_byte = context.create_function::<String, Vec<u8>, _>(|_context, file_path| -> Result<Vec<u8>, _> {
        //     file_util::load_file_byte(file_path).map_err(|_|{
        //         rlua::Error::UserDataBorrowMutError
        //     })
        // }).expect("load_file error");
        lua_file_util_table.set("loadFileByte", load_file_byte).unwrap();
    }

    {
        let load_file_string = context.create_function::<String, String, _>(|_context, file_path| -> Result<String, _> {
            file_util::load_file_string(file_path).map_err(|_|{
                rlua::Error::UserDataBorrowMutError
            })
        }).expect("load_file error");
        lua_file_util_table.set("loadFileString", load_file_string).unwrap();
    }

    {
        let fn_is_exist = context.create_function::<String, bool, _>(|_context, path| -> Result<bool, _> {
            // file_util::isExist()
            Ok(Path::new(&path).exists())
        }).expect("isExist error");
        lua_file_util_table.set("isExist", fn_is_exist).unwrap();
    }

    globals.set("__file_util", lua_file_util_table).unwrap();
}