use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::prelude::*;
use tokio::net::TcpStream;
use bstr::{BString, BStr};

use byteorder::{ByteOrder, LittleEndian};

use std::sync::{RwLock, Arc};
use lazy_static::lazy_static;
use crate::{channel_msg, network};
use std::net::{SocketAddr, ToSocketAddrs};


lazy_static! {

    static ref INS : RwLock<NetMgr> = RwLock::new(NetMgr {
        sender: None,
        receiver: None,
        network_list: HashMap::new(),
        channel_sender_clone: None,
    });
}

pub fn ins() -> &'static RwLock<NetMgr> {
    &INS
}


#[derive(Debug)]
pub enum NetworkChannelMsg {
    OnPeerAdd(i32),
    OnPeerDelete(i32),

    ConnectTcp(i32, String),
    CloseTcp(i32),

    // OnPeerConnected(i32),            直接传给 agent
    // OnPeerConnectError(i32),         直接传给 agent
    // OnPeerReceived(i32, BString),    直接传给 agent
    OnPeerSend(i32, BString),
    OnPeerClosed(i32, String),

}

static mut _SENDER: Option<mpsc::UnboundedSender<NetworkChannelMsg>> = None;

pub fn send(msg: NetworkChannelMsg) {
    unsafe {
        _SENDER.as_ref().unwrap().send(msg);
    }
}

type Tx = mpsc::UnboundedSender<NetworkChannelMsg>;

pub struct NetMgr {
    pub sender: Option<mpsc::UnboundedSender<NetworkChannelMsg>>,
    receiver: Option<mpsc::UnboundedReceiver<NetworkChannelMsg>>,
    pub network_list: HashMap<i32, Arc<NetPeer>>,
    pub channel_sender_clone: Option<mpsc::UnboundedSender<channel_msg::Msg>>,
}

pub fn init(channel_sender_clone: mpsc::UnboundedSender<channel_msg::Msg>) {
    {
        let x = ins().write();
        let mut y = x.unwrap();
        let (tx, rx) = mpsc::unbounded_channel::<NetworkChannelMsg>();
        y.sender = Some(tx.clone());
        y.receiver = Some(rx);
        y.channel_sender_clone = Some(channel_sender_clone);

        unsafe {
            _SENDER = Some(tx.clone());
        }
    }

    // unsafe{
    //     let a = ins();
    //     let mut x = a.write();
    //     let mut y = x.as_mut().unwrap();
    //     let z = y as &'static mut RwlockWriteGuard<NetMgr>;
    //     z.run();
    // }
    let a = ins();
    let mut x = a.write();
    let mut y = x.as_mut().unwrap();
    y.run()
}

impl NetMgr {
    pub fn run(&mut self) {
        let mut receiver = self.receiver.take().unwrap();
        let channel_sender = self.channel_sender_clone.as_ref().unwrap().clone();

        /**
            spawn 里面不能直接用 self ，不然 self 得指定 lifetime，比如 'static
        */
        tokio::spawn(async move {
            let is_exit = &mut false;
            loop {
                if *is_exit { break; }
                tokio::select! {
                    Some(msg) = receiver.recv() => {
                        match msg {
                            NetworkChannelMsg::OnPeerAdd(id) => {
                                log::info!("OnPeerAdd: {}", id);
                                let a = ins();
                                let mut x = a.write();
                                x.unwrap().network_list.insert(id, Arc::new(NetPeer::new(id, channel_sender.clone())));
                            }
                            NetworkChannelMsg::OnPeerDelete(id) => {
                                let a = ins();
                                let mut x = a.write().unwrap();
                               x.network_list.remove(&id);
                            }

                            NetworkChannelMsg::ConnectTcp(id, address) => {
                                let mut p = None;
                                {
                                    let mut a = ins();
                                    let mut x = a.write().unwrap();
                                    let peer = x.network_list.get_mut(&id);
                                    if let Some(node) = peer {
                                        p = Some(node.clone())
                                        // tokio::task::spawn_blocking(||{ futures::executor::block_on(node.connect(address))});

                                        // let res = tokio::task::spawn_blocking(move || {
                                        //   futures::executor::block_on(node.connect(address))
                                        // }).await;
                                        //

                                        // futures::executor::block_on(node.connect(address));
                                   }
                                }
                                if let Some(node) = p {
                                    node.connect(address).await;
                                }
                                // log::info!("ConnectTcp finished...");
                            }
                            NetworkChannelMsg::CloseTcp(id) => {
                                let mut p = None;
                                {
                                    let a = ins();
                                    let mut x = a.write().unwrap();
                                    let peer = x.network_list.get_mut(&id);
                                    if let Some(node) = peer {
                                        p = Some(node.clone())
                                   }
                                }
                                if let Some(node) = p {
                                    node.close().await;
                                }
                                // log::info!("CloseTcp finished...");
                            }
                            NetworkChannelMsg::OnPeerSend(id, mut data) => {
                                log::trace!("start on peer send");
                                let mut p2 = None;
                                {
                                    let a2 = ins();
                                    let mut x2 = a2.write().unwrap();
                                    let peer2 = x2.network_list.get_mut(&id);
                                    if let Some(node) = peer2 {
                                        p2 = Some(node.clone())
                                   }
                                }
                                if let Some(node) = p2 {
                                    node.write(data).await;
                                }
                                 log::trace!("finished on peer send");
                                // log::info!("ConnectTcp finished...");
                            }
                            _ =>{
                                log::debug!( "Error: not matched");
                            }
                        }
                    }
                }
            }
            log::debug!("NetMgr close ...");
        });
    }
}

pub struct NetPeer {
    pub id: i32,
    pub tcp_writer: Mutex<Option<WriteHalf::<TcpStream>>>,
    pub tcp_reader: Mutex<Option<ReadHalf::<TcpStream>>>,
    pub channel_sender_clone: mpsc::UnboundedSender<channel_msg::Msg>,
}

impl NetPeer {
    pub fn new(id: i32, channel_sender_clone: mpsc::UnboundedSender<channel_msg::Msg>) -> NetPeer {
        let ret = NetPeer {
            id,
            tcp_writer: Mutex::new(None),
            tcp_reader: Mutex::new(None),
            channel_sender_clone,
        };
        return ret;
    }
    pub async fn connect(&self, mut address: String) {
        let id = self.id;
        let channel_sender_for_receiver = self.channel_sender_clone.clone();
        let mut tcp_writer_temp = None;
        {
            let mut tcp_write_guard = self.tcp_writer.lock().await;
            if tcp_write_guard.is_some() {
                tcp_writer_temp = tcp_write_guard.take();
            }
        }

        if let Some(mut xxxx) = tcp_writer_temp {
            xxxx.shutdown().await;
        }

        log::debug!("do connect: {}", address);
        let address_parse_result = address.parse::<SocketAddr>();
        let mut addr;
        if let Ok(temp_addr) = address_parse_result {
            addr = temp_addr.to_string()
        }else{
            // 尝试解析dns
            let list :Vec<_>= address.split(":").collect();
            let dns_rst = dns_lookup::lookup_host(list[0]).unwrap();
            addr = format!("{}:{}", dns_rst[0], list[1]);
            // log::info!("host => addr: {}", &addr);
        }

        let stream = TcpStream::connect(addr).await;
        // let stream = futures::executor::block_on(TcpStream::connect(addr));
        if let Err(ref e) = stream {
            log::error!("{}", e);
            self.channel_sender_clone.send(channel_msg::Msg::OnTcpConnectError(id));
            // return Ok(());
            return;
        }
        log::info!("NetPeer {}, connect success {}",id, address);
        let stream = stream.unwrap();
        self.channel_sender_clone.send(channel_msg::Msg::OnTcpConnected(id));

        // stream.shutdown(std::net::Shutdown::Both);
        let rr: Option<ReadHalf<TcpStream>>;
        {
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
            let mut tcp_writer_guard = self.tcp_writer.lock().await;
            *tcp_writer_guard = Some(tw);

            rr = Some(tr);
        }
        tokio::spawn(async move {
            let mut head_buf = [0; 10];
            let mut r = rr.unwrap();
            loop {
                let receive_result = r.read_exact(&mut head_buf).await;
                if let Err(e) = receive_result {
                    log::debug!("NetPeer OnTcpClosed {}", id);
                    channel_sender_for_receiver.send(channel_msg::Msg::OnTcpClosed(id, e.to_string()));
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


                    try_handle_error!(channel_sender_for_receiver.send(channel_msg::Msg::OnTcpReceived(id, final_data)));
                } else {
                    log::error!("error in received len = 0")
                }
            }
        });
    }

    pub async fn close(&self) {
        let mut tcp_writer_temp = None;
        {
            let mut tcp_write_guard = self.tcp_writer.lock().await;

            if tcp_write_guard.is_some() {
                tcp_writer_temp = tcp_write_guard.take();
            }
        }

        if let Some(mut xxxx) = tcp_writer_temp {
            xxxx.shutdown().await;
        }
    }

    pub async fn write(&self, mut data: BString) {
        log::trace!("netpeer: {:?} OnTcpSend: len:{}", self.id,  data.len());
        // log::debug!("netpeer: {:?} OnTcpSend: len:{}, {:?}", self.id,  data.len(), data);

        let mut tcp_writer_temp = None;
        let mut tcp_write_guard = self.tcp_writer.lock().await;

        if tcp_write_guard.is_some() {
            tcp_writer_temp = tcp_write_guard.as_mut();
        }

        if let Some(mut tcp_writer) = tcp_writer_temp {
            // let mut v :[u8; 4]= [0; 4];
            // LittleEndian::write_u32(&mut v, data.len() as u32);
            // try_handle_error!(futures::executor::block_on(tcp_writer.write_all(0_u16.to_le_bytes().as_ref())));
            tcp_writer.write_all(((data.len() + 2) as u32).to_le_bytes().as_ref()).await;
            //
            // log::info!("test lock.....................");
            // let xx = self.tcp_writer.lock().await;
            // log::info!("test lock..................... after");
            let calc_value = network::util::calc_sum(data.as_slice());
            // log::debug!("calcSum(data.as_slice())  {}", calc_value);
            futures::executor::block_on(tcp_writer.write_all(calc_value.to_le_bytes().as_ref()));
            network::util::process_data(data.as_mut_slice());
            tcp_writer.write_all(data.as_ref()).await;
        }
    }
}
