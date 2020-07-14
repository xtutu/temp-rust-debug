use std::collections::HashMap;

use crate::channel_msg;
use tokio::sync::mpsc;
use std::sync::{Mutex, Arc};
use lazy_static::lazy_static;


lazy_static! {
    static ref INS : Mutex<State> = Mutex::new(State {
        agent_list: HashMap::new(),
    });
}

pub fn ins() -> &'static Mutex<State> {
    &INS
}

type Tx = mpsc::UnboundedSender<channel_msg::Msg>;

pub struct State {
    pub agent_list: HashMap<String, Tx>
}

impl State {
    pub fn update(&mut self) {
        for (_, tx) in self.agent_list.iter_mut() {
            let result = tx.send(channel_msg::Msg::Update);
            if let Err(e) = result {
                log::error!("{}", e);
            }
        }
    }

    pub fn add(&mut self, name: String, tx: Tx) {
        self.agent_list.insert(name, tx);
    }

    pub fn remove(&mut self, name: &String) {
        self.agent_list.remove(name);
    }
}



