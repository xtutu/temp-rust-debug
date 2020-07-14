#[cfg(test)]
mod tests{
    use xkit::file_util;
    use std::time::SystemTime;
    use chrono::prelude::*;
    #[test]
    fn test_file_util() {
        println!("--{}--", file_util::is_exist("./src"));
        println!("--{}--", file_util::is_exist("./src-xxx"));
    }
}