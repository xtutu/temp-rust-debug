use futures::io::Error;
use rlua::Table;
use bstr::{BString, BStr};
use bstr::{ByteSlice, ByteVec};
pub fn register(context: rlua::Context) {
    let globals = context.globals();
    let lua_xt_table = context.create_table().unwrap();


    // {
    //     let load_file_string = context.create_function::<String, String, _>(|_context, file_path| -> Result<String, _> {
    //         file_util::load_file_string(file_path).map_err(|_|{
    //             rlua::Error::UserDataBorrowMutError
    //         })
    //     }).expect("load_file error");
    //     lua_xt_table.set("MD5NyString", load_file_string).unwrap();
    // }

    globals.set("__XT", lua_xt_table).unwrap();
}