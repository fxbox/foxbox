/// Tests for parse.rs

extern crate thinkerbell;
use thinkerbell::parse::Parser;

extern crate serde_json;

#[test]
fn test_parse_empty_script() {
    let src = "{
      \"requirements\": [],
      \"allocations\": [],
      \"rules\": []
    }";

    let json : self::serde_json::Value = self::serde_json::from_str(src).unwrap();
    let result = Parser::parse(json);

    let script = match result {
        Ok(script) => script,
        Err(err) => panic!("Script was parsed incorrectly {:?}", err)
    };

    assert!(script.requirements.len() == 0);
    assert!(script.allocations.len() == 0);
    assert!(script.rules.len() == 0);
}

#[test]
fn test_parse_one_input() {
    let src = "{
      \"requirements\": [
        {
          \"kind\": \"clock\",
          \"inputs\": [\"ticks\"]
        },
        {
          \"kind\": \"display device\",
          \"outputs\": [\"show\"]
        }
      ],
      \"allocations\": [
        [
          \"built-in clock\"
        ],
        [
          \"built-in display 1\",
          \"built-in display 2\"
        ]
      ],
      \"rules\": [
        {
          \"condition\": [
            {
              \"input\": 0,
              \"capability\": \"ticks\",
              \"range\": [
                {\"value\": 3},
                null
              ]
            }
          ],
          \"action\": [
            {
              \"output\": 1,
              \"capability\": \"show\",
              \"args\": {
                \"reached\": true
              }
            }
          ]
        }
      ]
    }";

    let json : self::serde_json::Value = self::serde_json::from_str(src).unwrap();
    let result = Parser::parse(json);

    let script = match result {
        Ok(script) => script,
        Err(err) => panic!("Script was parsed incorrectly {:?}", err)
    };

}
    
