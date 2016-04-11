#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]
extern crate foxbox_taxonomy;
extern crate foxbox_thinkerbell;
extern crate serde;
extern crate transformable_channels;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use transformable_channels::mpsc::*;

use foxbox_thinkerbell::fake_env::FakeEnv;
use foxbox_thinkerbell::manager::*;

use foxbox_taxonomy::api::User;
use foxbox_taxonomy::util::Id;

fn load_json(path: &str) -> String {
    let mut file = File::open(path).unwrap();
    let mut source = String::new();
    file.read_to_string(&mut source).unwrap();
    source
}

#[test]
fn test_database_add_remove_script() {
    let (tx_env, _) = channel();
    let env = FakeEnv::new(Box::new(tx_env));

    let (tx, _) = channel();
    let mut db = ScriptManager::new(env, Path::new("./test_script_database.sqlite"), Box::new(tx)).unwrap();

    db.remove_all().unwrap();

    let name = Id::<ScriptId>::new("Sample Ruleset");
    db.put(&name, &load_json("./examples/ruleset.json"), &User::Id(1)).unwrap();
    let (_, owner) = db.get_source_and_owner(&name).unwrap();
    assert_eq!(owner, User::Id(1));
    db.set_enabled(&name, true).unwrap();
    assert_eq!(db.get_running_count(), 1);
    db.set_enabled(&name, false).unwrap();
    assert_eq!(db.get_running_count(), 0);
    db.set_enabled(&name, true).unwrap();
    assert_eq!(db.get_running_count(), 1);
    db.remove(&name).unwrap();
    assert_eq!(db.get_running_count(), 0);
}
