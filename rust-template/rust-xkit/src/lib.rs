pub mod time_util;
pub mod macros;
pub mod util;
pub mod file_util;


// 子模块是私有的
mod error;


// 这些是重导出函数
pub use error::CommonError;






#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
        try_handle_error_with_ok!(Ok(1))
    }
}
