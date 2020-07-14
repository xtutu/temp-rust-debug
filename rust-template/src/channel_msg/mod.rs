use bstr::BString;

#[derive(Debug)]
pub enum Msg {
    Update,

    OnTcpConnected(i32), // network_mgr 会传这个消息给 lua_agent
    OnTcpConnectError(i32), // network_mgr 会传这个消息给 lua_agent
    OnTcpReceived(i32, BString),// network_mgr 会传这个消息给 lua_agent
    // OnTcpSend(i32, BString),
    OnTcpClosed(i32, String),       // network_mgr 会传这个消息给 lua_agent

    // 通过 channel 来，从而避免死锁
    LuaCallRust(RustFn),

    CtrlC,

    Exit(i32)
}

#[derive(Debug)]
pub enum RustFn{
    ConnectTcp(i32, String),
    CloseTcp(i32)
}