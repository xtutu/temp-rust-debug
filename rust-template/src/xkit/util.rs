use std::collections::BTreeMap;
use serde_yaml::Value;
use std::path::Path;
use std::io;
use std::{fs, path};

pub fn combine_yaml_file<P: AsRef<Path>>(path_default: P, path_override: P) -> Result<String, serde_yaml::Error> {
    let default_content = fs::read_to_string(path_default).expect("combine_yaml_file error");
    if path_override.as_ref().exists() {
        let override_content = fs::read_to_string(path_override).expect("combine_yaml_file error");
        return combine_yaml(default_content.as_ref(), override_content.as_str());
    }
    return Ok(default_content)
}


pub fn combine_yaml(content1: &str, content2: &str) -> Result<String, serde_yaml::Error> {
    let mut deserialized_map: BTreeMap<String, Value> = serde_yaml::from_str(&content1)?;
    let deserialized_2: BTreeMap<String, Value> = serde_yaml::from_str(&content2)?;
    for (k, v) in deserialized_2 {
        if deserialized_map.contains_key(&k) {
            // 这个代码实际上放到if外面的话，就可以变成合并信息（包括 temp 没有的字段，也会合并进去）
            // 放在 if 里面则表示： 用于覆盖temp已有的字段
            let x = deserialized_map.entry(k.to_string()).or_insert(Value::Null);
            *x = v
        }
    }
    serde_yaml::to_string(&deserialized_map)
}


#[cfg(test)]
mod tests {
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
        let ret = super::combine_yaml(s1, s2).expect("");
        dbg!(ret);
    }
}