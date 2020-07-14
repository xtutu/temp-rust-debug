use futures::io::Error;
use rlua::Table;
use bstr::{BString, BStr};
use bstr::{ByteSlice, ByteVec};
use crate::network::network_mgr;

pub fn register(context: rlua::Context) {
    let globals = context.globals();
    let lua_net_peer_table = context.create_table().unwrap();

    {
        let fn_new_peer = context.create_function::<i32, (), _>(|_context, id| -> Result<(), _> {
            network_mgr::send(network_mgr::NetworkChannelMsg::OnPeerAdd(id));
            Ok(())
        }).expect("fn_new_peer error");
        lua_net_peer_table.set("newPeer", fn_new_peer).unwrap();


        let fn_connect_tcp = context.create_function::<(i32, String), (), _>(|_context, (id, address)| -> Result<(), _> {
            network_mgr::send(network_mgr::NetworkChannelMsg::ConnectTcp(id, address));
            Ok(())
        }).expect("fn_connect_tcp error");
        lua_net_peer_table.set("connectTcp", fn_connect_tcp).unwrap();


        let fn_close_tcp = context.create_function::<i32, (), _>(|_context, id| -> Result<(), _> {
            log::info!("in closeTcp");
            network_mgr::send(network_mgr::NetworkChannelMsg::CloseTcp(id));
            Ok(())
        }).expect("fn_close_tcp error");
        lua_net_peer_table.set("closeTcp", fn_close_tcp).unwrap();


        let fn_send = context.create_function::<(i32, BString), (), _>(|_context, (id, data)| -> Result<(), _> {
            network_mgr::send(network_mgr::NetworkChannelMsg::OnPeerSend(id, data));
            Ok(())
        }).expect("fn_send error");
        lua_net_peer_table.set("sendMsg", fn_send).unwrap();
    }



    globals.set("__net_peer", lua_net_peer_table).unwrap();
}