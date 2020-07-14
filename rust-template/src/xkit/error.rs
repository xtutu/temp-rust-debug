use std::error;
use std::error::Error;
use core::fmt;

#[derive(Debug)]
pub struct CommonError {
    info: String
}

impl CommonError {
    pub fn new(info: String) -> CommonError {
        CommonError {
            info
        }
    }
}

impl fmt::Display for CommonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.info)
    }
}

impl error::Error for CommonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}