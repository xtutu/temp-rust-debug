#[cfg(test)]
mod tests{
    use xkit::time_util;
    use std::time::SystemTime;
    use chrono::prelude::*;
    #[test]
    fn test_format() {
        println!("--{}--", time_util::format_now("%Y-%m-%d %H:%M:%S %z"));
        let now: DateTime<Utc> = Utc::now();
        let a = time_util::get_current_millisecond();
        let b = now.timestamp_millis();
        println!("{}", now);
        println!("{}, {}, {}", a, b, a == b);
        println!("{}",  Utc::now());
    }
}