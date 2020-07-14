use bstr::BString;

#[derive(Debug)]
pub enum Msg {
    Update,

    OnTcpConnected,
    OnTcpConnectError,
    OnTcpReceived(BString),
    OnTcpSend(BString),
    OnTcpClosed(String),


    LuaCallRust(RustFn),

    CtrlC,

    Exit(i32)
}

#[derive(Debug)]
pub enum RustFn{
    ConnectTcp(String),
    CloseTcp
}