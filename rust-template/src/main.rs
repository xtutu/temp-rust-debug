#![allow(unused_doc_comments)]
#![allow(dead_code, unused_mut)]
#![allow(unused_must_use)]
#![allow(unused_imports)]

#[macro_use]
extern crate xkit;

mod conf;
mod lua_util;
mod core;
mod agent;
mod state_mgr;
mod channel_msg;
mod network;

use std::error::Error;
use std::path;
use log;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use crate::core::event_loop::main_loop;
use tokio::sync::mpsc;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let result = init();
    if let Err(e) = result {
        log::error!("{}", e);
        return Err(e);
    }
    //
    {
        let cfg = conf::ins().read().unwrap();
        if cfg.with_compile {
            xkit::file_util::try_remove_path("dist", false);
            xkit::file_util::for_each_file("code", |e| {
                let mut path_name = e.to_str().unwrap().to_string();
                path_name = path_name.replace("\\", "/");
                if !path_name.ends_with(".lua") {
                    return;
                }
                // log::info!("{}", path_name);
                lua_util::compile(path_name);
            });
        }

        let mut state = state_mgr::ins().lock().unwrap();
        // let mut agent_list = Vec::<agent::LuaAgent>::new();
        let count = 1;
        for i in 1..=count {
            let name = "name-".to_string().add(&i.to_string());
            let start = xkit::time_util::get_current_millisecond();
            let mut agent = agent::LuaAgent::new(name.clone());


            let end = xkit::time_util::get_current_millisecond();
            log::debug!("init finished {}/{}   cost:{}", i, count, end - start);
            let result = agent.run();
            if let Err(e) = result {
                log::error!("{}", e);
            }
            state.add(name, agent.sender.clone());
        }
    }

    // try_lua3();
    let (tx, rx) = mpsc::unbounded_channel::<channel_msg::Msg>();
    /**
        其实也可以专门启动一个 std::thread 去跑 eventloop，里面用 basic_scheduler 来 select。
        在 ctrl-c 之后，就给 select 发送消息，然后用 waitGroup 来等待 std::thread 结束
    */
    let handle = main_loop(rx);

    tokio::signal::ctrl_c().await?;
    log::info!("receive ctrl c");
    tx.send(channel_msg::Msg::CtrlC);
    handle.await;

    return Ok(());
}

fn init() -> Result<(), Box<dyn Error>> {
    let root_path = "./";

    // init log4rs
    {
        let conf_default_path = path::Path::new(root_path).join("res").join("conf").join("log4rs_default.yaml");
        let conf_override_path = path::Path::new(root_path).join("res").join("conf").join("log4rs.yaml");
        let default_config_str = xkit::util::combine_yaml_file(conf_default_path, conf_override_path)?;
        // // result.map_err()
        // if let Err(e) = result{
        //     eprintln!("error {}", e);
        //     return Err(Box::new(xkit::CommonError::new("xxx".to_string())));
        // }
        let raw_config: log4rs::file::RawConfig = serde_yaml::from_str(default_config_str.as_str()).unwrap();

        let (append_list, errors) = raw_config.appenders_lossy(&Default::default());
        for error in &errors {
            eprintln!("{}", error);
        }

        let (config, errors) = log4rs::config::Config::builder()
            .appenders(append_list)
            .loggers(raw_config.loggers())
            .build_lossy(raw_config.root());
        for error in &errors {
            eprintln!("{}", error);
        }

        log4rs::init_config(config);
        // log4rs::init_file(path::Path::new(root_path).join("res").join("conf").join("log4rs.yaml"), Default::default())?;
    }

    conf::init(root_path);
    Ok(())
}