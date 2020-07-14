use std::sync::Arc;

use rlua::{
    AnyUserData, ExternalError, Function, Lua, MetaMethod, String, UserData, UserDataMethods,
};

#[test]
fn test_user_data() {
    struct UserData1(i64);
    struct UserData2(Box<i64>);

    impl UserData for UserData1 {};
    impl UserData for UserData2 {};

    Lua::new().context(|lua| {
        let userdata1 = lua.create_userdata(UserData1(1)).unwrap();
        let userdata2 = lua.create_userdata(UserData2(Box::new(2))).unwrap();

        assert!(userdata1.is::<UserData1>());
        assert!(!userdata1.is::<UserData2>());
        assert!(userdata2.is::<UserData2>());
        assert!(!userdata2.is::<UserData1>());

        assert_eq!(userdata1.borrow::<UserData1>().unwrap().0, 1);
        assert_eq!(*userdata2.borrow::<UserData2>().unwrap().0, 2);
    });
}

#[test]
fn test_methods() {
    struct MyUserData(i64);

    impl UserData for MyUserData {
        fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
            methods.add_method("get_value", |_, data, ()| Ok(data.0));
            methods.add_method_mut("set_value", |_, data, args| {
                data.0 = args;
                Ok(())
            });
        }
    }

    Lua::new().context(|lua| {
        let globals = lua.globals();
        let userdata = lua.create_userdata(MyUserData(42)).unwrap();
        globals.set("userdata", userdata.clone()).unwrap();
        lua.load(
            r#"
                function get_it()
                    return userdata:get_value()
                end

                function set_it(i)
                    return userdata:set_value(i)
                end
            "#,
        )
        .exec()
        .unwrap();
        let get = globals.get::<_, Function>("get_it").unwrap();
        let set = globals.get::<_, Function>("set_it").unwrap();
        assert_eq!(get.call::<_, i64>(()).unwrap(), 42);
        userdata.borrow_mut::<MyUserData>().unwrap().0 = 64;
        assert_eq!(get.call::<_, i64>(()).unwrap(), 64);
        set.call::<_, ()>(100).unwrap();
        assert_eq!(get.call::<_, i64>(()).unwrap(), 100);
    });
}

#[test]
fn test_metamethods() {
    #[derive(Copy, Clone)]
    struct MyUserData(i64);

    impl UserData for MyUserData {
        fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
            methods.add_method("get", |_, data, ()| Ok(data.0));
            methods.add_meta_function(
                MetaMethod::Add,
                |_, (lhs, rhs): (MyUserData, MyUserData)| Ok(MyUserData(lhs.0 + rhs.0)),
            );
            methods.add_meta_function(
                MetaMethod::Sub,
                |_, (lhs, rhs): (MyUserData, MyUserData)| Ok(MyUserData(lhs.0 - rhs.0)),
            );
            methods.add_meta_method(MetaMethod::Index, |_, data, index: String| {
                if index.to_str()? == "inner" {
                    Ok(data.0)
                } else {
                    Err("no such custom index".to_lua_err())
                }
            });
        }
    }

    Lua::new().context(|lua| {
        let globals = lua.globals();
        globals.set("userdata1", MyUserData(7)).unwrap();
        globals.set("userdata2", MyUserData(3)).unwrap();
        assert_eq!(
            lua.load("userdata1 + userdata2")
                .eval::<MyUserData>()
                .unwrap()
                .0,
            10
        );
        assert_eq!(
            lua.load("userdata1 - userdata2")
                .eval::<MyUserData>()
                .unwrap()
                .0,
            4
        );
        assert_eq!(lua.load("userdata1:get()").eval::<i64>().unwrap(), 7);
        assert_eq!(lua.load("userdata2.inner").eval::<i64>().unwrap(), 3);
        assert!(lua.load("userdata2.nonexist_field").eval::<()>().is_err());
    });
}

#[test]
fn test_gc_userdata() {
    struct MyUserdata {
        id: u8,
    }

    impl UserData for MyUserdata {
        fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
            methods.add_method("access", |_, this, ()| {
                assert!(this.id == 123);
                Ok(())
            });
        }
    }

    Lua::new().context(|lua| {
        lua.globals()
            .set("userdata", MyUserdata { id: 123 })
            .unwrap();

        assert!(lua
            .load(
                r#"
                    local tbl = setmetatable({
                        userdata = userdata
                    }, { __gc = function(self)
                        -- resurrect userdata
                        hatch = self.userdata
                    end })

                    tbl = nil
                    userdata = nil  -- make table and userdata collectable
                    collectgarbage("collect")
                    hatch:access()
                "#,
            )
            .exec()
            .is_err());
    });
}

#[test]
fn detroys_userdata() {
    struct MyUserdata(Arc<()>);

    impl UserData for MyUserdata {}

    let rc = Arc::new(());

    let lua = Lua::new();
    lua.context(|lua| {
        lua.globals()
            .set("userdata", MyUserdata(rc.clone()))
            .unwrap();
    });

    assert_eq!(Arc::strong_count(&rc), 2);
    drop(lua); // should destroy all objects
    assert_eq!(Arc::strong_count(&rc), 1);
}

#[test]
fn user_value() {
    struct MyUserData;
    impl UserData for MyUserData {}

    Lua::new().context(|lua| {
        let ud = lua.create_userdata(MyUserData).unwrap();
        ud.set_user_value("hello").unwrap();
        assert_eq!(ud.get_user_value::<String>().unwrap(), "hello");
        assert!(ud.get_user_value::<u32>().is_err());
    });
}

#[test]
fn test_functions() {
    struct MyUserData(i64);

    impl UserData for MyUserData {
        fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
            methods.add_function("get_value", |_, ud: AnyUserData| {
                Ok(ud.borrow::<MyUserData>()?.0)
            });
            methods.add_function("set_value", |_, (ud, value): (AnyUserData, i64)| {
                ud.borrow_mut::<MyUserData>()?.0 = value;
                Ok(())
            });
            methods.add_function("get_constant", |_, ()| Ok(7));
        }
    }

    Lua::new().context(|lua| {
        let globals = lua.globals();
        let userdata = lua.create_userdata(MyUserData(42)).unwrap();
        globals.set("userdata", userdata.clone()).unwrap();
        lua.load(
            r#"
                function get_it()
                    return userdata:get_value()
                end

                function set_it(i)
                    return userdata:set_value(i)
                end

                function get_constant()
                    return userdata.get_constant()
                end
            "#,
        )
        .exec()
        .unwrap();
        let get = globals.get::<_, Function>("get_it").unwrap();
        let set = globals.get::<_, Function>("set_it").unwrap();
        let get_constant = globals.get::<_, Function>("get_constant").unwrap();
        assert_eq!(get.call::<_, i64>(()).unwrap(), 42);
        userdata.borrow_mut::<MyUserData>().unwrap().0 = 64;
        assert_eq!(get.call::<_, i64>(()).unwrap(), 64);
        set.call::<_, ()>(100).unwrap();
        assert_eq!(get.call::<_, i64>(()).unwrap(), 100);
        assert_eq!(get_constant.call::<_, i64>(()).unwrap(), 7);
    });
}
