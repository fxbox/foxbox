/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

// Macros to help with json serializing of undeclared structs.
// json_value!({ }) returns a serde_json::value::Value from an anonymous struct.
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
            use std::collections::BTreeMap;
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

#[cfg(test)]
#[macro_use]
describe! json {
    before_each {
        extern crate serde_json;
    }

    describe! objects {
        it "should support string attributes" {
            assert_eq!(json!({ foo: "bar" }), r#"{"foo":"bar"}"#);
        }

        it "should support numbers" {
            assert_eq!(json!({ val: 1.02 }), r#"{"val":1.02}"#);
        }

        it "should support mixed types" {
            assert_eq!(json!({ type: "light", value: 1.2, on: true }),
                        r#"{"on":true,"type":"light","value":1.2}"#);
        }

        it "should support arrays" {
            assert_eq!(json!({ array: vec![1, 2, 3]}), r#"{"array":[1,2,3]}"#);
        }

        it "should support sub-objects" {
            assert_eq!(json!({ type: "complex", sub: json_value!({ a: 1, b: "foo" }) }),
                        r#"{"sub":{"a":1,"b":"foo"},"type":"complex"}"#);
        }

        it "should support sub-sub-objects" {
            assert_eq!(json!({ one: json_value!({ two: json_value!({ three: 3 }) }) }),
                        r#"{"one":{"two":{"three":3}}}"#);
        }
    }

    describe! arrays {
        it "should support numbers" {
            assert_eq!(json!([1, 100, 1000]), r#"[1,100,1000]"#);
        }

        it "should support sub-arrays" {
            assert_eq!(json!(["one", "two", json_value!(["three", "four"]), 5]),
                        r#"["one","two",["three","four"],5]"#);
        }

        it "should support sub-sub-arrays" {
            assert_eq!(json!(["one", json_value!(["two", json_value!(["three"]) ]) ]),
                        r#"["one",["two",["three"]]]"#);
        }

        it "should support mixed types" {
            assert_eq!(json!(["one", 2, 3.33, false]), r#"["one",2,3.33,false]"#);
        }

        it "should support objects" {
            assert_eq!(json!([ json_value!({foo: "bar"}) ]), r#"[{"foo":"bar"}]"#);
        }
    }
}
