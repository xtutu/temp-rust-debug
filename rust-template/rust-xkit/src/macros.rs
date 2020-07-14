#[macro_export]
macro_rules! try_handle_error {
    ($e:expr) => (match $e {
        Ok(val) => val,
        Err(err) =>  {
            log::error!("{:?}", err);
            return
        },
    });
}

#[macro_export]
macro_rules! try_handle_error_with_ok {
    ($e:expr) => (match $e {
        Ok(val) => val,
        Err(err) =>  {
            log::error!("{:?}", err);
            return Ok(())
        },
    });
}