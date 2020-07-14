use log;
use std::thread;
use tokio::time::{self, Duration};
use tokio::sync::mpsc;
use crate::state_mgr;
use crate::channel_msg;
use tokio::task::JoinHandle;

/**
    todo 貌似是 async 函数，如果没有加 await 来调用，那么就完全无效
        如果不是 async，那么里面的Spawn可以直接接生效的。
*/
pub fn main_loop(mut rx : mpsc::UnboundedReceiver::<channel_msg::Msg>)->JoinHandle<()>{
//    let mut delay = time::delay_for(Duration::from_millis(1000));
    // let mut interval_2 = time::interval(Duration::from_millis(20000));
//    let (mut tx, mut rx) = mpsc::channel(100);
//     interval.tick().poll

    log::info!("mainloop start");
    let ret = tokio::spawn(async move {
        // let rx = rx;
        let mut interval = time::interval(Duration::from_millis(10));
        loop {

            // 通过 channel， 居然不是在一个线程里执行！
            /**
                其实的确并不关心 await 回来之后，是否是回到同一线程中执行。（除了 UI 这种需要主线程刷新的）

                Futures 是对计算的抽象，它们描述了「做什么（what）」，但是与「在哪儿（where）」、「什么时候（when）」执行是分开的。

                哪怕 NodeJS 这种单线程的 await，依旧有并发，处理顺序的问题，那么这个时候就是需要 消息队列。
                或者 抢单这种，可以通过 updateDB.ByIdAndModifyId(newDoc) == 1 这种方式来，判断是否需要重试。
            */
            tokio::select! {
                _ = interval.tick()=> {
                    // log::debug!("interval   thread:{:?}", thread::current().id());
                    let state = state_mgr::ins();
                    state.lock().unwrap().update();
                }
                Some(msg) = rx.recv() => {
                    match msg {
                        channel_msg::Msg::CtrlC => {
                            break;
                        }
                         _ =>{
                        }
                    }
                }
            }
        }
        log::info!("mainloop exit");
    });
    return ret;

}