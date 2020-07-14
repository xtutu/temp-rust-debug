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
use crate::xkit::file_util;
use log;
use std::io::Cursor;
use byteorder::{ByteOrder, LittleEndian};
use bstr::{BString, BStr};
use bstr::{ByteSlice, ByteVec};
use std::ops::Index;
use crate::network;
use crate::xkit;

#[derive(Default)]
pub struct LuaAgentEntity {
    pub agent_uuid: String,
    pub tcp_writer: Option<WriteHalf::<TcpStream>>,
    pub lua_state: Option<Lua>,


    // pub luaFnOnReceivedTcpMsg: Option<Function<'static>>
}

pub struct LuaAgent {
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
                            try_handle_error_with_ok!(tx_for_tcp_sender.send(channel_msg::Msg::LuaCallRust(channel_msg::RustFn::ConnectTcp(address))));
                            Ok(())
                        }).expect("bind connectTcp error");
                        globals.set("connectTcp", fn_connect_tcp).unwrap();
                    }
                };
            }

            {
                let tx_for_tcp_sender = tx.clone();
                state_entity.lua_state.as_ref().unwrap().context(move |context| {
                    let globals = context.globals();
                    let fn_send_tcp_msg = context.create_function::<BString, (), _>(move |_context, msg| {
                        // log::debug!("sendTcpMsg  {:?}", msg);
                        try_handle_error_with_ok!(tx_for_tcp_sender.send(channel_msg::Msg::OnTcpSend(msg)));
                        Ok(())
                    }).expect("bind sendTcpMsg error");
                    globals.set("sendTcpMsg", fn_send_tcp_msg).unwrap();
                });
            }
            {
                // let entity_for_close_tcp = ret.entity.clone();
                let tx_for_close_tcp = tx.clone();
                state_entity.lua_state.as_ref().unwrap().context(move |context| {
                    let globals = context.globals();
                    let fn_close_tcp = context.create_function::<(), (), _>(move |_context, _| {
                        try_handle_error_with_ok!(tx_for_close_tcp.send(channel_msg::Msg::LuaCallRust(channel_msg::RustFn::CloseTcp)));
                        Ok(())
                    }).expect("bind closeTcp error");
                    globals.set("closeTcp", fn_close_tcp).unwrap();
                });
            }
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
                if let Some(_) = path_name.find(&(lua_root_dir.to_string() + "/prepare_require")){
                    return;
                }
                if let Some(_) = path_name.find(&(lua_root_dir.to_string() + "/_")){
                    return;
                }
                if let Some(_) = path_name.find(&(lua_root_dir.to_string() + "/.")) {
                    return;
                }

                // log::info!("{}", path_name);
                if let Some(idx) = path_name.find(&lua_root_dir) {
                    path_name = path_name[idx + lua_root_dir.len()+1..path_name.len() - 4].to_string();
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
                log::error!("{:?}", e)
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

        let channel_sender = self.sender.clone();
        tokio::spawn(async move {
            let is_exit = &mut false;
            loop {
                if *is_exit { break; }
                tokio::select! {
                    Some(msg) = receiver.recv() => {
                        match msg {
                            channel_msg::Msg::Update => {
                                let mut e = entity.lock().unwrap();
                                // log::debug!("agent: {:?} update {:?}", e.agent_uuid, thread::current().id());
                                e.lua_state.as_mut().unwrap().context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("update").expect("update function not found");
                                    let ret = lua_fn.call::<(), ()>(());
                                    if let Err(e) = ret {
                                        log::error!("{:?}", e)
                                    }
                                });
                            }

                            channel_msg::Msg::OnTcpConnected => {
                                let mut e = entity.lock().unwrap();
                                log::debug!("agent: {:?} OnTcpConnected", e.agent_uuid);
                                e.lua_state.as_mut().unwrap().context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("OnTcpConnected").expect("OnTcpConnected function not found");
                                    let ret = lua_fn.call::<(), ()>(());
                                    if let Err(e) = ret {
                                        log::error!("{:?}", e)
                                    }
                                });
                            }
                             channel_msg::Msg::OnTcpConnectError => {
                                let e = entity.lock();
                                let e = e.as_ref().unwrap();
                                log::debug!("agent: {:?} OnTcpConnectError", e.agent_uuid);
                                e.lua_state.as_ref().unwrap().context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("OnTcpConnectError").expect("OnTcpConnectError function not found");
                                    let ret = lua_fn.call::<(), ()>(());
                                    if let Err(e) = ret {
                                        log::error!("{:?}", e)
                                    }
                                });
                            }

                            channel_msg::Msg::OnTcpSend(mut data) => {
                                let mut e = entity.lock();
                                let e = e.as_mut().unwrap();
                                log::trace!("agent: {:?} OnTcpSend: len:{}, {:?}", e.agent_uuid,  data.len(), data);
                                let tcp_writer = e.tcp_writer.as_mut().unwrap();
                                // let mut v :[u8; 4]= [0; 4];
                                // LittleEndian::write_u32(&mut v, data.len() as u32);
                                // try_handle_error!(futures::executor::block_on(tcp_writer.write_all(0_u16.to_le_bytes().as_ref())));
                                try_handle_error!(futures::executor::block_on(tcp_writer.write_all(( (data.len() + 2) as u32) .to_le_bytes().as_ref())));

                                let calc_value = network::util::calc_sum(data.as_slice());
                                // log::debug!("calcSum(data.as_slice())  {}", calc_value);
                                futures::executor::block_on(tcp_writer.write_all(calc_value.to_le_bytes().as_ref()));
                                network::util::process_data(data.as_mut_slice());
                                // log::debug!("processData  {:?}", &data);
                                try_handle_error!(futures::executor::block_on(tcp_writer.write_all(data.as_ref())));

                                // let mut e = entity.lock().unwrap();
                                // log::debug!("agent: {:?} OnTcpSend:{:?}", e.name,  data);
                                // let mut tcp_writer = e.tcp_writer.as_mut().unwrap();
                                // tcp_writer.write_all(data.as_ref()).await;
                            }

                            channel_msg::Msg::OnTcpReceived(data) => {
                                let mut e = entity.lock();
                                let e = e.as_mut().unwrap();
                                log::trace!("agent: {:?} OnTcpReceived:{:?}", e.agent_uuid,  data);
                                e.lua_state.as_mut().unwrap().context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("OnTcpMsgReceived").expect("OnTcpMsgReceived function not found");
                                    let ret = lua_fn.call::<BString, ()>(data);
                                    if let Err(e) = ret {
                                        log::error!("{:?}", e)
                                    }
                                });
                            }

                            channel_msg::Msg::OnTcpClosed(reason)=> {
                                let mut e = entity.lock();
                                let e = e.as_mut().unwrap();
                                log::debug!("agent: {:?} OnTcpClosed", e.agent_uuid);
                                e.lua_state.as_mut().unwrap().context(|context| {
                                    let globals = context.globals();
                                    let lua_fn: Function = globals.get("OnTcpClosed").expect("OnTcpClosed function not found");
                                    let ret = lua_fn.call::<String, ()>(reason);
                                    if let Err(e) = ret {
                                        log::error!("{:?}", e)
                                    }
                                });
                            }

                            channel_msg::Msg::LuaCallRust(rust_fn)=>{
                                match rust_fn {
                                    channel_msg::RustFn::CloseTcp =>{
                                        let entity_clone = entity.clone();
                                        // let channel_sender_clone = channel_sender.clone();
                                        let mut agent_entity_wrap = entity_clone.lock();

                                        let agent_entity = agent_entity_wrap.as_mut().unwrap();

                                        // log::info!("111111111111 close tcp  has tcp_writer:{}", agent_entity.tcp_writer.is_some());
                                        // if let Some(ref t) = agent_entity.tcp_writer {
                                        if let Some(ref _temp_tcp_writer) = agent_entity.tcp_writer {
                                            // log::debug!("222222222222222 try drop tcp_writer_temp CloseTcp");
                                            let tcp_writer_temp = agent_entity.tcp_writer.take();
                                            futures::executor::block_on(tcp_writer_temp.unwrap().shutdown());
                                            // drop(tcp_writer_temp);
                                            // channel_sender_clone.send(channel_msg::Msg::OnTcpClosed("close by self".to_string()));
                                        }
                                    }
                                    channel_msg::RustFn::ConnectTcp(address)=>{
                                        let entity_clone = entity.clone();
                                        let channel_sender_clone = channel_sender.clone();
                                        tokio::spawn(async move {
                                            // 如果当前连着的话，直接断开
                                            {
                                                let mut agent_entity_wrap = entity_clone.lock();
                                                let agent_entity = agent_entity_wrap.as_mut().unwrap();
                                                // log::info!("close pre connect 111   {}", agent_entity.tcp_writer.is_some());
                                                if let Some(ref _temp_tcp_writer) = agent_entity.tcp_writer {
                                                    let tcp_writer_temp = agent_entity.tcp_writer.take();
                                                    // log::debug!("try drop tcp_writer_temp ConnectTcp");
                                                    futures::executor::block_on(tcp_writer_temp.unwrap().shutdown());
                                                    // tcp_writer_temp.unwrap().shutdown();
                                                    // log::info!("close pre connect 2222");
                                                    // channel_sender_clone.send(channel_msg::Msg::OnTcpClosed("close by self".to_string()));
                                                }
                                            }
                                            log::debug!("do connect: {}", address);
                                            let addr = address.parse::<SocketAddr>().expect("parse error");
                                            let stream = TcpStream::connect(addr).await;
                                            if let Err(ref e) = stream {
                                                log::error!("{}", e);
                                                channel_sender_clone.send(channel_msg::Msg::OnTcpConnectError);
                                                // return Ok(());
                                                return;
                                            }
                                            let stream = stream.unwrap();
                                            channel_sender_clone.send(channel_msg::Msg::OnTcpConnected);

                                            // stream.shutdown(std::net::Shutdown::Both);
                                            let rr: Option<ReadHalf<TcpStream>>;
                                            // let mut w: Option<WriteHalf<TcpStream>> = None;
                                            {
                                                let mut agent_entity_wrap = entity_clone.lock();
                                                let agent_entity = agent_entity_wrap.as_mut().unwrap();
                                                // 这个是有效的
                                                // stream.shutdown(std::net::Shutdown::Both);
                                                // 这个也是有效的
                                                // stream.shutdown(std::net::Shutdown::Write);

                                                // 这样貌似也是可以的？不过这里可能是因为 stream 本身被 drop 了
                                                // let mut stream = stream;
                                                // let (mut tr , mut tw ) = stream.split();
                                                // tw.shutdown();

                                                let (tr, mut tw) = tokio::io::split(stream);
                                                // tw.poll_shutdown()
                                                // tw shutdown 没有用
                                                 // tw.shutdown();
                                                agent_entity.tcp_writer = Some(tw);

                                                rr = Some(tr);
                                            }



                                            let channel_sender_for_receiver = channel_sender_clone.clone();
                                            tokio::spawn(async move {
                                                let mut head_buf = [0; 10];
                                                let mut r = rr.unwrap();
                                                loop {
                                                    let receive_result = r.read_exact(&mut head_buf).await;
                                                    if let Err(e) = receive_result {
                                                        // log::info!("error in read ");
                                                        channel_sender_for_receiver.send(channel_msg::Msg::OnTcpClosed(e.to_string()));
                                                        break;
                                                    }
                                                    let received_len = receive_result.unwrap();

                                                    // 实际上应该是不需要检查的
                                                    if received_len != 10 {
                                                        log::debug!("unused check {}", received_len)
                                                    }
                                                    let len = LittleEndian::read_u32(&head_buf[0..4]) as usize;
                                                    let msg_id = LittleEndian::read_u16(&head_buf[4..6]);
                                                    let guess_size = LittleEndian::read_i32(&head_buf[6..10]);
                                                    let guess_size = guess_size as usize;

                                                    log::trace!("len {} msg_id {} guess_size {}", len, msg_id, guess_size);

                                                    let data_size = len - 6;
                                                    // let mut data= Vec::<u8>::with_capacity(data_size).resize();
                                                    let mut data = vec![0; data_size];
                                                    // log::debug!("data size {} {}", data.as_mut_slice().len(), data_size);
                                                    if len > 0 {
                                                        let received_result = r.read_exact(data.as_mut_slice()).await;
                                                        let received = try_handle_error!(received_result);
                                                        if received != data_size {
                                                            log::error!("received size error {} {}", received, data_size);
                                                            break;
                                                        }

                                                        let final_data: BString;
                                                        if guess_size == 0 {
                                                            final_data = BString::from(data);
                                                        } else {
                                                            let result = lz4::block::decompress(data.as_slice(), Some(guess_size as i32));
                                                            if let Err(e) = result {
                                                                log::error!("decompress error {:}", e);
                                                                break;
                                                            }
                                                            final_data = BString::from(result.unwrap());
                                                        }


                                                        try_handle_error!(channel_sender_for_receiver.send(channel_msg::Msg::OnTcpReceived(final_data)));
                                                    } else {
                                                        log::error!("error in received len = 0")
                                                    }
                                                }
                                            });
                                        });
                                    }
                                }
                            }

                            channel_msg::Msg::Exit(v) => {
                                *is_exit = true;
                                let e = entity.lock().unwrap();
                                log::debug!("agent: {:?} exit({})", e.agent_uuid, v);
                                return;
                            }

                            _ =>{
                                let e = entity.lock().unwrap();
                                log::debug!("agent: {:?} Error: not matched", e.agent_uuid);
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

