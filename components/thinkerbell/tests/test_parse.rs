extern crate fxbox_thinkerbell;
extern crate fxbox_taxonomy;
extern crate serde_json;

use fxbox_thinkerbell::ast::*;
use fxbox_thinkerbell::parse::*;
use fxbox_thinkerbell::values::*;
use fxbox_thinkerbell::util::*;

use fxbox_taxonomy::requests::*;

#[test]
fn test_parse_bad_field() {
    let src = "{
      \"requirements\": [],
      \"allocations\": [],
      \"rules\": []
    }".to_owned();

    let result = Parser::parse(src);
    match result {
        Err(serde_json::error::Error::SyntaxError(serde_json::error::ErrorCode::UnknownField(field), _, _)) => {
            assert!(field == "requirements".to_owned() || field == "allocations".to_owned())
        },
        _ => assert!(false)
    };
}

#[test]
fn test_parse_empty() {
    let src = "{ \"rules\": []}".to_owned();
    let script = Parser::parse(src).unwrap();
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
}".to_owned();
    Parser::parse(src).unwrap();
}

