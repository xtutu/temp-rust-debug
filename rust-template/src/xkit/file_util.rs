use std::{fs, io};

use walkdir::{WalkDir, DirEntry};
use std::path::{PathBuf, Path};

pub fn load_file_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path)
}

pub fn load_file_byte<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    fs::read(path)
}

pub fn for_each_file<F>(dir: &str, f: F) where F: Fn(PathBuf) {
    for entry in WalkDir::new(dir).follow_links(true).into_iter().filter_map(|e| {
        if let Ok(ret) = e {
            if ret.file_type().is_file() {
                return Some(ret);
            } else {
                // log::warn!("is not file {:?}", ret);
            }
        }
        None
    }) {
        // println!("{}", entry.path().display());
        f(entry.into_path())
    }
}

pub fn format_path(path: String) -> String {
    path.replace("\\", "/")
}

pub fn try_remove_path(path: &str, is_file_path: bool)-> io::Result<()> {
    let path_info= Path::new(path);
    if !path_info.exists(){
        return Ok(())
    }
    if is_file_path{
        fs::remove_file(path)
    }else{
        fs::remove_dir_all(path)
    }
}

pub fn try_create_dir(path: &str, is_file_path: bool)-> io::Result<()> {
    let mut dir_path = path;
    if is_file_path{
        let mut file_path = path.replace("\\", "/");
        let idx = file_path.rfind("/");
        if idx == None {
            log::info!("error path:{:?}", path);
            panic!()
        }
        dir_path = &file_path[0..idx.unwrap()];
        // log::info!("dir_path: {:?}", dir_path);
        return fs::create_dir_all(dir_path);
    }
    // log::info!("dir_path: {:?}", dir_path);
    return fs::create_dir_all(dir_path)
}