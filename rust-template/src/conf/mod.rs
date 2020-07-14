use serde::{Serialize, Deserialize};
use std::sync::{Mutex, Arc, RwLock};
use std::{fs, path};
use serde_yaml::Value;
use lazy_static::lazy_static;
use crate::xkit;

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Conf {
    #[serde(rename = "chatId")]
    pub chat_id: String,
    #[serde(rename = "clientPath")]
    pub client_path: String,
    #[serde(rename = "serverPath")]
    pub server_path: String,
    #[serde(rename = "withTimeInfo")]
    pub with_time_info: String,
    #[serde(rename = "isDebug")]
    pub is_debug: bool,

    #[serde(rename = "withCompile")]
    pub with_compile: bool,
    #[serde(rename = "useDist")]
    pub use_dist: bool,

    #[serde(skip)]
    pub root_dir: String,
}


// 实际上本身 static 就是为了全局引用。所以没必要 Arc。
// 之所以可以通过 &Mutex 可以访问，是因为 & 本身是 copy 语义。所以很完美
// 那为什么需要 Arc？ 因为如果直接用 & 是需要标注生命周期的！因为这里是全局的单例，所以刚好可以直接用 `static !!!
////////////////////
// 为什么需要 Rc 呢？为什么不直接用 &？
// 因为& 是引用，这就涉及到生命周期的问题！而Rc则不用考虑这点，相当于独立的Copy。


// lazy_static! {
//     static ref INS : Arc<Mutex<Conf>> = Arc::new(Mutex::new(Conf::default()));
// }

lazy_static! {
    static ref INS : RwLock<Conf> = RwLock::new(Conf::default());
}


//static Ins
pub fn init(root_path: &str) {
    let conf_default_path = path::Path::new(root_path).join("res").join("conf").join("conf_default.yaml");
    let conf_override_path = path::Path::new(root_path).join("res").join("conf").join("conf.yaml");


    *INS.write().unwrap() = serde_yaml::from_str(xkit::util::combine_yaml_file(conf_default_path, conf_override_path).unwrap().as_str()).unwrap();

    {
        let mut cfg = INS.write().unwrap();
        cfg.root_dir = root_path.to_string();
    }
    log::debug!("{:?}", &*&*(INS.read().unwrap()));
}

/**
usage:
    let confArc = conf::get_instance();
    let conf = confArc.lock().unwrap();
why?:
    https://stackoverflow.com/questions/54056268/temporary-value-is-freed-at-the-end-of-this-statement
*/
pub fn ins() -> &'static RwLock<Conf> {
    &INS
}


//static mut INS: Option<Arc<Mutex<Conf>>> = None;
//
////static Ins
//pub fn init(root_path: &str) {
//    let conf_path = path::Path::new(root_path).join("res").join("conf").join("conf.yaml");
//    unsafe {
//        INS.get_or_insert_with(|| {
//            let contents = fs::read_to_string(conf_path).expect("error");
//            let p: Conf = serde_yaml::from_str(contents.as_str()).expect("xx");
////                dbg!(&p);
//            Arc::new(Mutex::new(p))
//        });
//    }
//}
//
// **
//    println!("aaa {}", cfg.lock().unwrap().chat_id); // ok
//    println!("bbb {}", cfg.lock().unwrap().chat_id); // ok
//
//    // 会死锁！
//    println!("ddd {0} {1}", cfg.lock().unwrap().chat_id, cfg.lock().unwrap().chat_id);
//*/
//pub fn get_instance() -> Arc<Mutex<Conf>> {
//    unsafe {
//        INS.as_ref().unwrap().clone()
//    }
//}