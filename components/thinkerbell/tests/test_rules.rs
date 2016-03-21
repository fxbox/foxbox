extern crate foxbox_taxonomy;
extern crate foxbox_thinkerbell;

extern crate transformable_channels;

use foxbox_thinkerbell::fake_env::*;
use foxbox_thinkerbell::parse::*;
use foxbox_thinkerbell::run::*;
use foxbox_thinkerbell::ast::*;

use foxbox_taxonomy::api::{ Error as APIError };
use foxbox_taxonomy::util::Id;
use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::values::{ OnOff, Range, Type, TypeError as APITypeError , Value };

use std::marker::PhantomData;
use std::thread;
use std::collections::{ HashMap, HashSet };

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
    let (tx_done, rx_done) = channel();
    let (tx_send, rx_send) = channel();

    let env = FakeEnv::new(tx_env);
    let mut exec = Execution::<FakeEnv>::new();

    thread::spawn(move || {
        for msg in rx {
            if let Event::Env(FakeEnvEvent::Done) = msg {
                tx_done.send(()).unwrap();
            } else if let Event::Env(FakeEnvEvent::Send { id, value }) = msg {
                tx_send.send((id, value)).unwrap();
            } else {
                println!("LOG: {:?}", msg);
            }
        }
    });

    let script_1 = Script {
        rules: vec![
            Rule {
                conditions: vec![
                    Match {
                        source: vec![
                            GetterSelector::new()
                        ],
                        kind: ChannelKind::OnOff,
                        range: Range::Eq(Value::OnOff(OnOff::On)),
                        duration: None,
                        phantom: PhantomData
                    }
                ],
                execute: vec![
                    Statement {
                        destination: vec![
                            SetterSelector::new()
                        ],
                        value: Value::OnOff(OnOff::Off),
                        kind: ChannelKind::OnOff,
                        phantom: PhantomData,
                    }
                ],
                phantom: PhantomData
            }
        ],
        phantom: PhantomData,
    };

    let adapter_id_1 = Id::<AdapterId>::new("Adapter 1");
    let service_id_1 = Id::<ServiceId>::new("Service 1");
    let getter_id_1 = Id::<Getter>::new("Getter 1");
    let getter_id_2 = Id::<Getter>::new("Getter 2");
    let setter_id_1 = Id::<Setter>::new("Setter 1");
    let setter_id_2 = Id::<Setter>::new("Setter 2");
    let setter_id_3 = Id::<Setter>::new("Setter 3");

    println!("* We can start executing a trivial rule.");
    exec.start(env.clone(), script_1, tx_run).unwrap();

    println!("* Changing the structure of the network doesn't break the rule.");
    env.execute(Instruction::AddAdapters(vec![adapter_id_1.to_string()]));
    rx_done.recv().unwrap();

    env.execute(Instruction::AddServices(vec![
        Service {
            id: service_id_1.clone(),
            adapter: adapter_id_1.clone(),
            getters: HashMap::new(),
            setters: HashMap::new(),
            tags: HashSet::new(),
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::AddGetters(vec![
        Channel {
            id: getter_id_1.clone(),
            adapter: adapter_id_1.clone(),
            service: service_id_1.clone(),
            tags: HashSet::new(),
            last_seen: None,
            mechanism: Getter {
                watch: true,
                poll: None,
                updated: None,
                trigger: None,
                kind: ChannelKind::OnOff,
            }
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::AddSetters(vec![
        Channel {
            id: setter_id_1.clone(),
            adapter: adapter_id_1.clone(),
            service: service_id_1.clone(),
            last_seen: None,
            tags: HashSet::new(),
            mechanism: Setter {
                push: None,
                updated: None,
                kind: ChannelKind::OnOff,
            }
        }
    ]));
    rx_done.recv().unwrap();

    println!("* Injecting the expected value triggers the send.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));

    rx_done.recv().unwrap();
    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_1);
    assert_eq!(value, Value::OnOff(OnOff::Off));

    println!("* Injecting an out-of-range value does not trigger the send.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));

    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    println!("* Injecting an error does not trigger the send.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Err(APIError::TypeError(APITypeError {
            expected: Type::OnOff,
            got: Type::OpenClosed
        })))
    ]));

    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    println!("* Injecting the expected value again triggers the send again.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));

    rx_done.recv().unwrap();
    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_1);
    assert_eq!(value, Value::OnOff(OnOff::Off));

    println!("* Adding a second getter doesn't break the world.");
    env.execute(Instruction::AddGetters(vec![
        Channel {
            id: getter_id_2.clone(),
            adapter: adapter_id_1.clone(),
            service: service_id_1.clone(),
            tags: HashSet::new(),
            last_seen: None,
            mechanism: Getter {
                watch: true,
                poll: None,
                updated: None,
                trigger: None,
                kind: ChannelKind::OnOff,
            }
        }
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    println!("* Changing the state of the second getter while the condition remains true with the second getter doesn't do anything.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    println!("* Changing the state of the first getter while the condition remains true with the second getter doesn't do anything.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    println!("* If neither condition is met, the second getter can trigger the send.");

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_1);
    assert_eq!(value, Value::OnOff(OnOff::Off));

    println!("* If neither condition is met, the first getter can trigger the send.");

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_1);
    assert_eq!(value, Value::OnOff(OnOff::Off));

    println!("* If we add a second setter, it also receives these sends.");
    env.execute(Instruction::AddSetters(vec![
        Channel {
            id: setter_id_2.clone(),
            adapter: adapter_id_1.clone(),
            service: service_id_1.clone(),
            last_seen: None,
            tags: HashSet::new(),
            mechanism: Setter {
                push: None,
                updated: None,
                kind: ChannelKind::OnOff,
            }
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();

    let events : HashMap<_, _> = (0..2).map(|_| {
        rx_send.recv().unwrap()
    }).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(*events.get(&setter_id_1).unwrap(), Value::OnOff(OnOff::Off));
    assert_eq!(*events.get(&setter_id_2).unwrap(), Value::OnOff(OnOff::Off));
    assert!(rx_send.try_recv().is_err());

    println!("* If we add a setter of a mismatched type, it does not receive these sends.");
    env.execute(Instruction::AddSetters(vec![
        Channel {
            id: setter_id_3.clone(),
            adapter: adapter_id_1.clone(),
            service: service_id_1.clone(),
            last_seen: None,
            tags: HashSet::new(),
            mechanism: Setter {
                push: None,
                updated: None,
                kind: ChannelKind::Ready,
            }
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    assert!(rx_send.try_recv().is_err());

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();

    let events : HashMap<_, _> = (0..2).map(|_| {
        rx_send.recv().unwrap()
    }).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(*events.get(&setter_id_1).unwrap(), Value::OnOff(OnOff::Off));
    assert_eq!(*events.get(&setter_id_2).unwrap(), Value::OnOff(OnOff::Off));
    assert!(rx_send.try_recv().is_err());

    println!("");
}