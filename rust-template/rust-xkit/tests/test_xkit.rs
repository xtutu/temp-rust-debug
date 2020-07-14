#[cfg(test)]
mod tests {
    use xkit::util;

    use std::io::{self, BufRead, BufReader, Write};
    #[test]
    fn test_yaml_value() {
        let s1 = r#"---
chatId: 111
scheduleAt: "0 0 20 * * *"
withTimeInfo: "MM/dd hh:mm" # MM/dd hh:mm   （空字符串表示不按时间排序）
isDebug: false"#;

        let s2 = r#"---
chatId: 222
isDebug: true"#;
        let ret = util::combine_yaml(s1, s2).expect("");
        dbg!(ret);
    }
}