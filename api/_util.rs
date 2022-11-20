use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use woothee::parser::WootheeResult;

#[derive(Serialize, Deserialize, Debug)]
pub struct UAResult<'a> {
    pub name: &'a str,
    pub category: &'a str,
    pub os: &'a str,
    pub os_version: Cow<'a, str>,
    pub browser_type: &'a str,
    pub version: &'a str,
    pub vendor: &'a str,
}

impl<'a> From<WootheeResult<'a>> for UAResult<'a> {
    fn from(def: WootheeResult<'a>) -> UAResult<'a> {
        UAResult {
            name: def.name,
            category: def.category,
            os: def.os,
            os_version: def.os_version,
            browser_type: def.browser_type,
            version: def.version,
            vendor: def.vendor,
        }
    }
}

impl From<UAResult<'_>> for Map<String, Value> {
    fn from(ua_result: UAResult<'_>) -> Self {
        let mut map = Map::new();
        map.insert("name".to_owned(), Value::String(ua_result.name.to_owned()));
        map.insert(
            "category".to_owned(),
            Value::String(ua_result.category.to_owned()),
        );
        map.insert("os".to_owned(), Value::String(ua_result.os.to_owned()));
        map.insert(
            "os_version".to_owned(),
            Value::String(ua_result.os_version.to_string()),
        );
        map.insert(
            "browser_type".to_owned(),
            Value::String(ua_result.browser_type.to_owned()),
        );
        map.insert(
            "version".to_owned(),
            Value::String(ua_result.version.to_owned()),
        );
        map.insert(
            "vendor".to_owned(),
            Value::String(ua_result.vendor.to_owned()),
        );
        map
    }
}
