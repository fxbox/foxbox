extern crate foxbox_thinkerbell;
extern crate transformable_channels;

use foxbox_thinkerbell::fake_env::*;
use foxbox_thinkerbell::parse::*;
use foxbox_thinkerbell::run::*;
use foxbox_thinkerbell::ast::Script;

use std::thread;

use transformable_channels::mpsc::*;

#[derive(Debug)]
enum Event {
    Env(FakeEnvEvent),
    Run(ExecutionEvent),
}

#[test]
fn test_compile() {
    let (tx, rx) : (_, Receiver<Event>)= channel();

    let tx_env = Box::new(tx.map(|event| Event::Env(event)));
    let tx_run = tx.map(|event| Event::Run(event));

    let env = FakeEnv::new(tx_env);
    let mut exec = Execution::<FakeEnv>::new();

    thread::spawn(move || {
        for msg in rx {
            println!("LOG: {:?}", msg)
        }
    });

    println!("* Attempting to parse an run an empty script will raise an error.");
    let script = Script::parse("{\"rules\": []}").unwrap();
    match exec.start(env, script, tx_run) {
        Err(Error::CompileError(CompileError::SourceError(SourceError::NoRule))) => {},
        other => panic!("Unexpected result {:?}", other)
    }

    println!("//FIXME: Attempting to parse a script with an empty condition will raise an error.");
    println!("//FIXME: Attempting to parse a script with an empty statement will raise an error.");
    println!("//FIXME: Attempting to parse a script with an empty source will raise an error.");
    println!("//FIXME: Attempting to parse a script with an empty destination will raise an error.");
    println!("//FIXME: Attempting to parse a script with a type error in a match will raise an error.");
    println!("//FIXME: Attempting to parse a script with a type error in a send will raise an error.");

    println!("");
}

#[test]
fn test_run() {
    let (tx, rx) : (_, Receiver<Event>)= channel();

    let tx_env = Box::new(tx.map(|event| Event::Env(event)));
    let tx_run = tx.map(|event| Event::Run(event));

    let env = FakeEnv::new(tx_env);
    let mut exec = Execution::<FakeEnv>::new();

    thread::spawn(move || {
        for msg in rx {
            println!("LOG: {:?}", msg)
        }
    });

/*
    println!("* Attempting to parse an run an empty script will raise an error.");
    let script = Parser::parse("{\"rules\": []}".to_owned()).unwrap();
    match exec.start(env, script, tx_run) {
        Err(Error::CompileError(CompileError::SourceError(SourceError::NoRule))) => {},
        other => panic!("Unexpected result {:?}", other)
    }

    println!("//FIXME: Attempting to parse a script with an empty condition will raise an error.");
    println!("//FIXME: Attempting to parse a script with an empty statement will raise an error.");
    println!("//FIXME: Attempting to parse a script with an empty source will raise an error.");
    println!("//FIXME: Attempting to parse a script with an empty destination will raise an error.");
    println!("//FIXME: Attempting to parse a script with a type error in a match will raise an error.");
    println!("//FIXME: Attempting to parse a script with a type error in a send will raise an error.");
*/
    println!("");
}