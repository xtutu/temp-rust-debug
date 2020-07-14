use chrono::prelude::*;

pub fn now()->DateTime<FixedOffset>{
    let t: DateTime<Utc> = Utc::now();
    t.with_timezone(&FixedOffset::east(8 * 3600))
}

pub fn get_current_millisecond()->i64{
    now().timestamp_millis()
}

/**
    https://docs.rs/chrono/0.4.11/chrono/format/strftime/index.html#specifiers
*/
pub fn format_by_time(date: &DateTime<FixedOffset>, fmt_str: &str)->String{
    date.format(fmt_str).to_string()
}

pub fn format_now(fmt_str: &str)->String{
    let now: DateTime<FixedOffset> = now();
    format_by_time(&now, fmt_str)
}

#[cfg(test)]
mod tests{
    use std::time::SystemTime;
    use chrono::prelude::*;
    #[test]
    fn test_format() {
        println!("--{}--", super::format_now("%Y-%m-%d %H:%M:%S %z"));
        let now: DateTime<Utc> = Utc::now();
        let a = super::get_current_millisecond();
        let b = now.timestamp_millis();
        println!("{}", now);
        println!("{}, {}, {}", a, b, a == b);
        println!("{}",  super::Utc::now());
    }
}