extern crate foxbox_thinkerbell;
extern crate foxbox_taxonomy;
extern crate serde_json;

use foxbox_thinkerbell::ast::*;
use foxbox_thinkerbell::parse::*;

#[test]
fn test_parse_bad_field() {
    let src = "{
      \"requirements\": [],
      \"allocations\": [],
      \"rules\": []
  }";

    let result = Script::parse(src);
    match result {
        Err(ParseError::UnknownFields {
            names: fields,
            ..
        }) => {
            assert!(fields.contains(&"requirements".to_owned()));
            assert!(fields.contains(&"allocations".to_owned()));
        },
        _ => assert!(false)
    };
}

#[test]
fn test_parse_empty() {
    let src = "{ \"rules\": []}";
    let script = Script::parse(src).unwrap();
    assert_eq!(script.rules.len(), 0);
}

#[test]
fn test_parse_simple_rule() {
    let src =
"{
  \"rules\": [
    {
      \"execute\": [],
      \"conditions\": [
      ]
    }
  ]
}";
    Script::parse(src).unwrap();
}

