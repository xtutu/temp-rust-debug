use xkit::time_util;
pub fn register(context: rlua::Context) {
    let globals = context.globals();
    let table_time_util = context.create_table().unwrap();

    {
        let lua_fn = context.create_function::<(), i64, _>(|_context, _| -> Result<i64, _> {
            Ok(time_util::get_current_millisecond())
        }).expect("lua_fn error");
        table_time_util.set("get_current_millisecond", lua_fn).unwrap();
    }

    globals.set("__time_util", table_time_util).unwrap();
}