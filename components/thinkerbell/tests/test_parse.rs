extern crate foxbox_thinkerbell;
extern crate foxbox_taxonomy;
extern crate serde_json;

use foxbox_thinkerbell::parse::*;

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

