use tokio;
use tokio::sync::mpsc;
use std::thread;
use std::num::Wrapping;
use std::error::Error;
use std::sync::{Arc, Mutex};
use crate::{conf, lua_util};
use crate::channel_msg;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::io::{ReadHalf, WriteHalf};
use rlua::{Lua, Function};
use xkit::file_util;
use log;
use std::io::Cursor;
use byteorder::{ByteOrder, LittleEndian};
use bstr::{BString, BStr};
use bstr::{ByteSlice, ByteVec};
use std::ops::Index;
use crate::network;
use xkit;

#[derive(Default)]
pub struct LuaAgentEntity {
    pub agent_uuid: String,
    // pub tcp_writer: Option<WriteHalf::<TcpStream>>,
    pub lua_state: Option<Lua>,
    pub last_gc_time: i64,

    // pub luaFnOnReceivedTcpMsg: Option<Function<'static>>
}

pub struct LuaAgent {
    // lua agent 的 loop 消息来源
    pub sender: mpsc::UnboundedSender<channel_msg::Msg>,
    receiver: Option<mpsc::UnboundedReceiver<channel_msg::Msg>>,

    /**
        Arc 应该只是在 clone 以及 drop 的时候，加个锁而已（锁的是自己的计数）
        Mutex 才是真正的数据锁
    */
    pub entity: Arc<Mutex<LuaAgentEntity>>,

    lua_context_unsafe: rlua::Context<'static>,
    // https://rust-lang.github.io/async-book/03_async_await/01_chapter.html
    // use the Mutex in futures::lock rather than the one from std::sync.
}

impl LuaAgent {
    pub fn new(agent_uuid: String) -> LuaAgent {
        let uuid_string = agent_uuid.clone();
        let (tx, rx) = mpsc::unbounded_channel::<channel_msg::Msg>();

        // 只能有一个 agent！
        network::network_mgr::init(tx.clone());

        let lua_agent_entity = LuaAgentEntity {
            agent_uuid,
            ..LuaAgentEntity::default()
        };

        // let start = xkit::time_util::get_current_millisecond();
        // let end = xkit::time_util::get_current_millisecond();
        // log::debug!("cost time: {}", end - start);
        let lua_state = lua_util::new_lua_state(uuid_string);


        let lua_context_temp = unsafe { lua_state.get_context() };
        let entity_temp = Arc::new(Mutex::new(lua_agent_entity));

        let ret = LuaAgent {
            sender: tx.clone(),
            receiver: Some(rx),
            entity: entity_temp.clone(),
            lua_context_unsafe: lua_context_temp,
        };


        let entity_wrap_clone = ret.entity.clone();
        let mut state_agent = entity_wrap_clone.lock();
        let xx = state_agent.as_mut();
        let state_entity = xx.unwrap();
        state_entity.lua_state = Some(lua_state);


        // bind network
        {
            {
                let tx_for_tcp_sender = tx.clone();
                // let entity_for_tcp_connect_clone = ret.entity.clone();
                let context = lua_context_temp;
                {
                    let globals = context.globals();
                    {
                        let fn_connect_tcp = context.create_function::<String, (), _>(move |_context, address| {
                            try_handle_error_with_ok!(tx_for_tcp_sender.send(channel_msg::Msg::LuaCallRust(channel_msg::RustFn::ConnectTcp(9999, address))));
                            Ok(())
                        }).expect("bind connectTcp error");
                        globals.set("connectTcp", fn_connect_tcp).unwrap();
                    }
                };
            }

            // {
            //     let tx_for_tcp_sender = tx.clone();
            //     state_entity.lua_state.as_ref().unwrap().context(move |context| {
            //         let globals = context.globals();
            //         let fn_send_tcp_msg = context.create_function::<BString, (), _>(move |_context, msg| {
            //             // log::debug!("sendTcpMsg  {:?}", msg);
            //             try_handle_error_with_ok!(tx_for_tcp_sender.send(channel_msg::Msg::OnTcpSend(9999, msg)));
            //             Ok(())
            //         }).expect("bind sendTcpMsg error");
            //         globals.set("sendTcpMsg", fn_send_tcp_msg).unwrap();
            //     });
            // }
            // {
            //     // let entity_for_close_tcp = ret.entity.clone();
            //     let tx_for_close_tcp = tx.clone();
            //     state_entity.lua_state.as_ref().unwrap().context(move |context| {
            //         let globals = context.globals();
            //         let fn_close_tcp = context.create_function::<(), (), _>(move |_context, _| {
            //             try_handle_error_with_ok!(tx_for_close_tcp.send(channel_msg::Msg::LuaCallRust(channel_msg::RustFn::CloseTcp(9999))));
            //             Ok(())
            //         }).expect("bind closeTcp error");
            //         globals.set("closeTcp", fn_close_tcp).unwrap();
            //     });
            // }
        }


        // 加载 lua ，并启动
        state_entity.lua_state.as_ref().unwrap().context(|context| {
            // lua_util::do_require_file(context, "prepare_require\\init".to_string());
            lua_util::do_require_file_by_cache(context, "prepare_require/init".to_string());


            let lua_root_dir = lua_util::get_root_dir();
            file_util::for_each_file(&lua_root_dir, |e| {
                let mut path_name = e.to_str().unwrap().to_string();
                path_name = path_name.replace("\\", "/");
                if !path_name.ends_with(".lua") {
                    return;
                }
                if let Some(_) = path_name.find(&(lua_root_dir.to_string() + "/prepare_require")) {
                    return;
                }
                if let Some(_) = path_name.find(&(lua_root_dir.to_string() + "/_")) {
                    return;
                }
                if let Some(_) = path_name.find(&(lua_root_dir.to_string() + "/.")) {
                    return;
                }

                // log::info!("{}", path_name);
                if let Some(idx) = path_name.find(&lua_root_dir) {
                    path_name = path_name[idx + lua_root_dir.len() + 1..path_name.len() - 4].to_string();
                }

                // log::info!("{}", path_name);
                // lua_util::do_require_file(context, path_name);
                lua_util::do_require_file_by_cache(context, path_name);
            });

            // lua_util::do_require_file(context, "main".to_string());
            lua_util::do_require_file_by_cache(context, "main".to_string());
            let globals = context.globals();
            let main_fn: Function = globals.get("main").expect("main function not found");
            let ret = main_fn.call::<(), ()>(());
            if let Err(e) = ret {
                let mut err_str = e.to_string();
                // if err_str.contains("runtime error"){
                //     // err_str = err_str.replace("\n", "aaaaaaaaaaaaaaa");
                // }
                log::error!("{}", err_str)
            }
        });


        ret
    }

    pub fn destroy(&self, v: i32) {
        try_handle_error!(self.sender.send(channel_msg::Msg::Exit(v)));
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut receiver = self.receiver.take().unwrap();
        // let agent_entity = self.entity.clone();
        // log::debug!("in running11111..");
        let entity = self.entity.clone();
        log::debug!("agent start {}..", entity.lock().unwrap().agent_uuid);

        // let channel_sender = self.sender.clone();
        tokio::spawn(async move {
            let is_exit = &mut false;
            loop {
                if *is_exit { break; }
                tokio::select! {
                    Some(msg) = receiver.recv() => {
                        match msg {
                            channel_msg::Msg::Update => {
                                let mut e = entity.lock().unwrap();
                                let mut need_do_gc = false;
                                let current_time = xkit::time_util::get_current_millisecond();
                                if current_time - e.last_gc_time > 10000{
                                    e.last_gc_time = current_time;
                                    need_do_gc = true;
                                }
                                // log::debug!("agent: {:?} update {:?}", e.agent_uuid, thread::current().id());
                                let lua_state =  e.lua_state.as_mut().unwrap();
                                if need_do_gc{
                                    lua_state.gc_collect();
                                }
                                lua_state.context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("update").expect("update function not found");
                                    let ret = lua_fn.call::<(), ()>(());
                                    if let Err(e) = ret {
                                        log::error!("{}", e)
                                    }
                                });
                            }

                            channel_msg::Msg::OnTcpConnected(id) => {
                                let mut e = entity.lock().unwrap();
                                log::trace!("agent: {:?} OnTcpConnected id:{}", e.agent_uuid, id);
                                e.lua_state.as_mut().unwrap().context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("OnTcpConnected").expect("OnTcpConnected function not found");
                                    let ret = lua_fn.call::<i32, ()>(id);
                                    if let Err(e) = ret {
                                        log::error!("{}", e)
                                    }
                                });
                            }
                             channel_msg::Msg::OnTcpConnectError(id) => {
                                let e = entity.lock();
                                let e = e.as_ref().unwrap();
                                log::debug!("agent: {:?} OnTcpConnectError id: {}", e.agent_uuid, id);
                                e.lua_state.as_ref().unwrap().context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("OnTcpConnectError").expect("OnTcpConnectError function not found");
                                    let ret = lua_fn.call::<i32, ()>(id);
                                    if let Err(e) = ret {
                                        log::error!("{}", e)
                                    }
                                });
                            }

                            // channel_msg::Msg::OnTcpSend(id, mut data) => {
                            //     // let mut e = entity.lock();
                            //     // let e = e.as_mut().unwrap();
                            //     // log::trace!("agent: {:?} OnTcpSend: len:{}, {:?}", e.agent_uuid,  data.len(), data);
                            //     // let tcp_writer = e.tcp_writer.as_mut().unwrap();
                            //     // // let mut v :[u8; 4]= [0; 4];
                            //     // // LittleEndian::write_u32(&mut v, data.len() as u32);
                            //     // // try_handle_error!(futures::executor::block_on(tcp_writer.write_all(0_u16.to_le_bytes().as_ref())));
                            //     // try_handle_error!(futures::executor::block_on(tcp_writer.write_all(( (data.len() + 2) as u32) .to_le_bytes().as_ref())));
                            //     //
                            //     // let calc_value = network::util::calc_sum(data.as_slice());
                            //     // // log::debug!("calcSum(data.as_slice())  {}", calc_value);
                            //     // futures::executor::block_on(tcp_writer.write_all(calc_value.to_le_bytes().as_ref()));
                            //     // network::util::process_data(data.as_mut_slice());
                            //     // // log::debug!("processData  {:?}", &data);
                            //     // try_handle_error!(futures::executor::block_on(tcp_writer.write_all(data.as_ref())));
                            //     //
                            //     // // let mut e = entity.lock().unwrap();
                            //     // // log::debug!("agent: {:?} OnTcpSend:{:?}", e.name,  data);
                            //     // // let mut tcp_writer = e.tcp_writer.as_mut().unwrap();
                            //     // // tcp_writer.write_all(data.as_ref()).await;
                            // }

                            channel_msg::Msg::OnTcpReceived(id, data) => {
                                let mut e = entity.lock();
                                let e = e.as_mut().unwrap();
                                log::trace!("agent: {:?} OnTcpReceived:{:?}", e.agent_uuid,  data);
                                e.lua_state.as_mut().unwrap().context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("OnTcpMsgReceived").expect("OnTcpMsgReceived function not found");
                                    let ret = lua_fn.call::<(i32,BString), ()>((id, data));
                                    if let Err(e) = ret {
                                        log::error!("{}", e)
                                    }
                                });
                            }

                            channel_msg::Msg::OnTcpClosed(id, reason)=> {
                                let mut e = entity.lock();
                                let e = e.as_mut().unwrap();
                                log::trace!("agent: {:?} OnTcpClosed", e.agent_uuid);
                                e.lua_state.as_mut().unwrap().context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("OnTcpClosed").expect("OnTcpClosed function not found");
                                    let ret = lua_fn.call::<(i32, String), ()>((id, reason));
                                    if let Err(e) = ret {
                                        log::error!("{}", e)
                                    }
                                });
                            }

                            // channel_msg::Msg::LuaCallRust(rust_fn)=>{
                            //     match rust_fn {
                            //         channel_msg::RustFn::CloseTcp(id) =>{
                            //         }
                            //         channel_msg::RustFn::ConnectTcp(id, address)=>{
                            //         }
                            //     }
                            // }

                            channel_msg::Msg::Exit(v) => {
                                *is_exit = true;
                                let e = entity.lock().unwrap();
                                log::debug!("agent: {:?} exit({})", e.agent_uuid, v);
                                return;
                            }

                            _ =>{
                                let e = entity.lock().unwrap();
                                log::debug!("agent: {:?} Error: not matched  {:?}", e.agent_uuid, msg);
                            }
                        }
                    }
                }
            }
            log::debug!("agent close {}..", entity.lock().unwrap().agent_uuid);
        });


        return Ok(());
    }
}

