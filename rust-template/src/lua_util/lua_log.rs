use rlua::{Function, Lua, MetaMethod, Result, UserData, UserDataMethods, Variadic, Error, MultiValue};

// #[derive(Copy, Clone)]
// struct LuaLog();
//
// impl UserData for LuaLog {
//     fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
//         // methods.add_method("magnitude", |_, vec, ()| {
//         //     let mag_squared = vec.0 * vec.0 + vec.1 * vec.1;
//         //     Ok(mag_squared.sqrt())
//         // });
//         //
//         // methods.add_meta_function(MetaMethod::Add, |_, (vec1, vec2): (Vec2, Vec2)| {
//         //     Ok(Vec2(vec1.0 + vec2.0, vec1.1 + vec2.1))
//         // });
//         // methods.add_function()
//     }
// }


pub fn register(context: rlua::Context) {
    let globals = context.globals();

    {
        let fn_debug = context.create_function::<String, (), _>(|_context, msg| {
            log::debug!(target:"lua", "{}", msg);
            Ok(())
        }).expect("fn_debug error");
        globals.set("fn_debug", fn_debug).unwrap();
    }

    {
        let fn_info = context.create_function::<String, (), _>(|_context, msg| {
            log::info!(target:"lua", "{}", msg);
            Ok(())
        }).expect("fn_info error");
        globals.set("fn_info", fn_info).unwrap();
    }


    {
        let fn_warn = context.create_function::<String, (), _>(|_context, msg| {
            log::warn!(target:"lua", "{}", msg);
            Ok(())
        }).expect("fn_warn error");
        globals.set("fn_warn", fn_warn).unwrap();
    }

    {
        let fn_error = context.create_function::<String, (), _>(|_context, msg| {
            log::error!(target:"lua", "{}", msg);
            Ok(())
        }).expect("fn_error error");
        globals.set("fn_error", fn_error).unwrap();
    }
}