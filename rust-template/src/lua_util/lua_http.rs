use rlua::Table;
use bstr::{BString, BStr};
use bstr::{ByteSlice, ByteVec};

pub fn register(context: rlua::Context) {
    let globals = context.globals();
    let lua_http_util_table = context.create_table().unwrap();

    {
        let fn_get = context.create_function::<String, String, _>(|_context, url| -> Result<String, _> {
            // let rsp = reqwest::blocking::get(&url);
            // let rsp = tokio::( ||reqwest::get(&url).await);
            // let rsp  = reqwest::get(&url).await;

            let rsp = attohttpc::get(url).send();
            if let Err(err) = rsp {
                return Ok(err.to_string());
            }
            let ret = rsp.unwrap().text();
            if let Err(err) = ret {
                return Ok(err.to_string());
            }
            return Ok(ret.unwrap());

        }).expect("fn_get error");

        lua_http_util_table.set("get", fn_get).unwrap();
    }

    globals.set("__http_util", lua_http_util_table).unwrap();
}