/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

// Macros to help with json serializing of undeclared structs.
// json_value!({ }) retunns a serde_json::value::Value from an anonymous struct.
// json!({ }) returns a string from an anonymous struct.
// Combining both macros let you build arbitrary complex json objects easily.
//
// Examples: json!({ type: "light", value: 1.2, on: true });
//           json!({ type: "notification", value: json_value!({ r: 54, g: 12, b: 88 }) });

#[macro_export]
macro_rules! json {
    ({ $($i:ident: $v:expr),* }) => {
        {
            use std::collections::BTreeMap;
            let mut map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            $(
                map.insert(String::from(stringify!($i)), serde_json::to_value(&$v));
            )*
            serde_json::to_string(&map).unwrap_or("{}".to_owned())
        }
    };

    ([ $($v:expr),* ]) => {
        {
            use std::collections::BTreeMap;
            let mut vec: Vec<serde_json::Value> = Vec::new();
            $(
                vec.push(serde_json::to_value(&$v));
            )*
            serde_json::to_string(&vec).unwrap_or("[]".to_owned())
        }
    }
}

#[macro_export]
macro_rules! json_value {
    ({ $($i:ident: $v:expr),* }) => {
        {
            let mut map: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            $(
                map.insert(String::from(stringify!($i)), serde_json::to_value(&$v));
            )*
            serde_json::to_value(&map)
        }
    };

    ([ $($v:expr),* ]) => {
        {
            let mut vec: Vec<serde_json::Value> = Vec::new();
            $(
                vec.push(serde_json::to_value(&$v));
            )*
            serde_json::to_value(&vec)
        }
    }
}


#[test]
fn test_json_macro() {
    // Object tests.
    assert_eq!(json!({ foo: "bar" }), "{\"foo\":\"bar\"}");
    assert_eq!(json!({ val: 1.02 }), "{\"val\":1.02}");
    assert_eq!(json!({ type: "light", value: 1.2, on: true }), "{\"on\":true,\"type\":\"light\",\"value\":1.2}");
    assert_eq!(json!({ array: vec![1, 2, 3]}), "{\"array\":[1,2,3]}");
    assert_eq!(json!({ type: "complex", sub: json_value!({ a: 1, b: "foo" }) }), "{\"sub\":{\"a\":1,\"b\":\"foo\"},\"type\":\"complex\"}");

    // Array tests.
    assert_eq!(json!([1, 100, 1000]), "[1,100,1000]");
    assert_eq!(json!(["one", "two", json_value!(["three", "four"]), 5]), "[\"one\",\"two\",[\"three\",\"four\"],5]");
    assert_eq!(json!(["one", 2, 3.33, false]), "[\"one\",2,3.33,false]");
}
