extern crate foxbox_thinkerbell;
extern crate foxbox_taxonomy;
extern crate serde_json;

use foxbox_taxonomy::parse::*;
use foxbox_thinkerbell::ast::*;

#[test]
fn test_parse_bad_field() {
    let src = "{
      \"name\": \"foo\",
      \"requirements\": [],
      \"allocations\": [],
      \"rules\": []
  }";

    Script::from_str(src).unwrap();
}

#[test]
fn test_parse_empty() {
    let src = "{ \"name\": \"foo\", \"rules\": []}";
    let script = Script::from_str(src).unwrap();
    assert_eq!(script.rules.len(), 0);
}

#[test]
fn test_parse_simple_rule() {
    let src =
"{
  \"name\": \"foo\",
  \"rules\": [
    {
      \"execute\": [],
      \"conditions\": [
      ]
    }
  ]
}";
    Script::from_str(src).unwrap();
}

