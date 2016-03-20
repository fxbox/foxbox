extern crate foxbox_thinkerbell;
extern crate transformable_channels;

use foxbox_thinkerbell::parse::*;
use foxbox_thinkerbell::run::*;
use foxbox_thinkerbell::simulator::*;

use std::thread;

use transformable_channels::mpsc::*;

#[derive(Debug)]
enum Event {
    Simulator(SimulatorEvent),
    Env(ExecutionEvent),
}

#[test]
fn test_start() {
    let (tx, rx) : (_, Receiver<Event>)= channel();

    let tx_env = Box::new(tx.map(|event| Event::Simulator(event)));
    let env = TestEnv::new(tx_env);
    let mut exec = Execution::<TestEnv>::new();

    thread::spawn(move || {
        for msg in rx {
            println!("LOG: {:?}", msg)
        }
    });

    println!("* Attempting to parse an run an empty script...");
    let script = Parser::parse("{\"rules\": []}".to_owned()).unwrap();
    let tx_simulator = tx.map(|event| Event::Env(event));
    match exec.start(env, script, tx_simulator) {
        Err(Error::CompileError(CompileError::SourceError(SourceError::NoRule))) => {},
        other => panic!("Unexpected result {:?}", other)
    }
}