extern crate foxbox_taxonomy;
extern crate foxbox_thinkerbell;

extern crate transformable_channels;

extern crate chrono;

use foxbox_thinkerbell::fake_env::*;
use foxbox_thinkerbell::run::*;
use foxbox_thinkerbell::ast::*;

use foxbox_taxonomy::api::{ Error as APIError, User };
use foxbox_taxonomy::io::*;
use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::values::{ Duration, OnOff, Range, TimeStamp, Type, TypeError as APITypeError , Value };

use std::fmt::Debug;
use std::marker::PhantomData;
use std::thread;
use std::collections::HashMap;

use transformable_channels::mpsc::*;

use chrono::{ UTC, Duration as ChronoDuration };

#[derive(Debug)]
enum Event {
    Env(FakeEnvEvent),
    Run(ExecutionEvent),
}

#[test]
fn test_compile() {
    println!("[test_compile] START");
    let (tx, rx) : (_, Receiver<Event>)= channel();

    let tx_env = Box::new(tx.map(|event| Event::Env(event)));
    let tx_run = tx.map(|event| Event::Run(event));

    let env = FakeEnv::new(tx_env);
    let mut exec = Execution::<FakeEnv>::new();

    thread::spawn(move || {
        for _msg in rx {
            // Can be useful for debugging, but that's generally noise.
            // println!("LOG: {:?}", _msg)
        }
    });

    println!("[test_compile] * Attempting to parse an run an empty script will raise an error.");
    let script = Script::from_str(r#"{"name": "foo", "rules": []}"#).unwrap();
    match exec.start(env, script, User::None, tx_run) {
        Err(Error::CompileError(CompileError::SourceError(SourceError::NoRule))) => {},
        other => panic!("Unexpected result {:?}", other)
    }

    println!("[test_compile] //FIXME: Attempting to parse a script with an empty condition will raise an error.");
    println!("[test_compile] //FIXME: Attempting to parse a script with an empty statement will raise an error.");
    println!("[test_compile] //FIXME: Attempting to parse a script with an empty source will raise an error.");
    println!("[test_compile] //FIXME: Attempting to parse a script with an empty destination will raise an error.");
    println!("[test_compile] //FIXME: Attempting to parse a script with a type error in a match will raise an error.");
    println!("[test_compile] //FIXME: Attempting to parse a script with a type error in a send will raise an error.");

    println!("[test_compile] END");
}

#[test]
fn test_run() {
    println!("[test_run] * START");
    let (tx, rx) : (_, Receiver<Event>) = channel();

    let tx_env = Box::new(tx.map(|event| Event::Env(event)));
    let tx_run = tx.map(|event| Event::Run(event));
    let (tx_done, rx_done) = channel();
    let (tx_send, rx_send) = channel();

    let env = FakeEnv::new(tx_env);
    let mut exec = Execution::<FakeEnv>::new();

    let data_off = Payload::from_value(&Value::OnOff(OnOff::Off), &Type::OnOff).unwrap();

    println!("[test_run] * Spawning thread.");
    thread::spawn(move || {
        println!("[test_run] In thread. rx = {:?}", rx);
        for msg in rx {
            println!("[test_run] * In for loop. msg = {:?}", msg);
            if let Event::Env(FakeEnvEvent::Done) = msg {
                tx_done.send(()).unwrap();
            } else if let Event::Env(FakeEnvEvent::Send { id, value }) = msg {
                tx_send.send((id, value)).unwrap();
            } else {
                // Can be useful for debugging, but that's generally noise.
                println!("LOG: {:?}", msg)
            }
        }
    });

    println!("[test_run] * Preparing script.");
    let script_1 = Script {
        name: "Test script".to_owned(),
        rules: vec![
            Rule {
                conditions: vec![
                    Match {
                        source: vec![
                            ChannelSelector::new()
                        ],
                        kind: ChannelKind::LightOn,
                        range: Range::Eq(Value::OnOff(OnOff::On)),
                        duration: None,
                        phantom: PhantomData
                    }
                ],
                execute: vec![
                    Statement {
                        destination: vec![
                            ChannelSelector::new()
                        ],
                        value: data_off,
                        kind: ChannelKind::LightOn,
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
    let getter_id_1 = Id::<Channel>::new("Getter 1");
    let getter_id_2 = Id::<Channel>::new("Getter 2");
    let setter_id_1 = Id::<Channel>::new("Setter 1");
    let setter_id_2 = Id::<Channel>::new("Setter 2");
    let setter_id_3 = Id::<Channel>::new("Setter 3");

    println!("[test_run] * We can start executing a trivial rule.");
    exec.start(env.clone(), script_1, User::None, tx_run).unwrap();

    println!("[test_run] * Changing the structure of the network doesn't break the rule.");
    env.execute(Instruction::AddAdapters(vec![adapter_id_1.to_string()]));
    rx_done.recv().unwrap();

    env.execute(Instruction::AddServices(vec![
        Service::empty(&service_id_1, &adapter_id_1)
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::AddChannels(vec![
        Channel {
            kind: ChannelKind::LightOn,
            supports_fetch: true,
            supports_watch: true,
            .. Channel::empty(&getter_id_1, &service_id_1, &adapter_id_1)
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::AddChannels(vec![
        Channel {
            kind: ChannelKind::LightOn,
            supports_send: true,
            .. Channel::empty(&setter_id_1, &service_id_1, &adapter_id_1)
        }
    ]));
    rx_done.recv().unwrap();

    println!("[test_run] * Injecting the expected value triggers the send.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));

    println!("[test_run] after env.execute(...);");
    rx_done.recv().unwrap();
    println!("[test_run] after rx_done.recv().unwrap();");
    // IT HANGS JUST THE LINE BELOW //////////////////////////////////////
    let (id, value) = rx_send.recv().unwrap();
    println!("[test_run] after rx_send.recv().unwrap();");
    assert_eq!(id, setter_id_1);
    println!("[test_run] after assert_eq!(id, setter_id_1);");
    assert_eq!(value, Value::OnOff(OnOff::Off));

    println!("[test_run] * Injecting an out-of-range value does not trigger the send.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));

    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    println!("[test_run] * Injecting an error does not trigger the send.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Err(APIError::TypeError(APITypeError {
            expected: Type::OnOff,
            got: Type::OpenClosed
        })))
    ]));

    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    println!("[test_run] * Injecting the expected value again triggers the send again.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));

    rx_done.recv().unwrap();
    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_1);
    assert_eq!(value, Value::OnOff(OnOff::Off));

    println!("[test_run] * Adding a second getter doesn't break the world.");
    env.execute(Instruction::AddChannels(vec![
        Channel {
            kind: ChannelKind::LightOn,
            supports_fetch: true,
            supports_watch: true,
            .. Channel::empty(&getter_id_2, &service_id_1, &adapter_id_1)
        }
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    println!("[test_run] * Changing the state of the second getter while the condition remains true with the second getter doesn't do anything.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    println!("[test_run] * Changing the state of the first getter while the condition remains true with the second getter doesn't do anything.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    println!("[test_run] * If neither condition is met, the second getter can trigger the send.");

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_1);
    assert_eq!(value, Value::OnOff(OnOff::Off));

    println!("[test_run] * If neither condition is met, the first getter can trigger the send.");

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_1);
    assert_eq!(value, Value::OnOff(OnOff::Off));

    println!("[test_run] * If we add a second setter, it also receives these sends.");
    env.execute(Instruction::AddChannels(vec![
        Channel {
            kind: ChannelKind::LightOn,
            supports_send: true,
            .. Channel::empty(&setter_id_2, &service_id_1, &adapter_id_1)
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

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
    rx_send.try_recv().unwrap_err();

    println!("[test_run] * If we add a setter of a mismatched type, it does not receive these sends.");
    env.execute(Instruction::AddChannels(vec![
        Channel {
            kind: ChannelKind::Ready,
            supports_send: true,
            .. Channel::empty(&setter_id_3, &service_id_1, &adapter_id_1)
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

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
    rx_send.try_recv().unwrap_err();

    println!("[test_run] * Removing a getter resets its condition_is_met to false.");
    env.execute(Instruction::RemoveChannels(vec![
        getter_id_1.clone()
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();

    let events : HashMap<_, _> = (0..2).map(|_| {
        rx_send.recv().unwrap()
    }).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(*events.get(&setter_id_1).unwrap(), Value::OnOff(OnOff::Off));
    assert_eq!(*events.get(&setter_id_2).unwrap(), Value::OnOff(OnOff::Off));
    rx_send.try_recv().unwrap_err();

    println!("[test_run] * Removing a setter does not prevent the other setter from receiving.");
    env.execute(Instruction::RemoveChannels(vec![
        setter_id_1.clone()
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();

    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_2);
    assert_eq!(value, Value::OnOff(OnOff::Off));
    rx_send.try_recv().unwrap_err();

    println!("[test_run] * Even if a setter has errors, other setters will receive the send.");
    env.execute(Instruction::AddChannels(vec![
        Channel {
            kind: ChannelKind::LightOn,
            supports_send: true,
            .. Channel::empty(&setter_id_1, &service_id_1, &adapter_id_1)
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectSetterErrors(vec![
        (setter_id_1.clone(), Some(APIError::TypeError(APITypeError {
            expected: Type::OnOff,
            got: Type::OpenClosed
        })))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();

    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_2);
    assert_eq!(value, Value::OnOff(OnOff::Off));
    rx_send.try_recv().unwrap_err();

    println!("[test_run] END");
}


fn sleep<T>(rx_done: &Receiver<()>, rx_send: &Receiver<(Id<Channel>, Value)>, rx_timer: &Receiver<T>)
    where T: Debug {
    thread::sleep(std::time::Duration::from_millis(100));
    rx_send.try_recv().unwrap_err();
    rx_done.try_recv().unwrap_err();
    while let Ok(msg) = rx_timer.try_recv() {
        // Consume rx_timer
        println!("...(consuming rx_timer {:?})", msg);
    }
    println!("...(sleep complete)");
}

#[test]
fn test_run_with_delay() {
{
    println!("[test_run_with_delay] * START");
    let (tx, rx) : (_, Receiver<Event>)= channel();

    let tx_env = Box::new(tx.map(|event| Event::Env(event)));
    let tx_run = tx.map(|event| Event::Run(event));
    let (tx_done, rx_done) = channel();
    let (tx_send, rx_send) = channel();
    let (tx_timer, rx_timer) = channel();

    let env = FakeEnv::new(tx_env);
    let mut exec = Execution::<FakeEnv>::new();

    let data_off = Payload::from_value(&Value::OnOff(OnOff::Off), &Type::OnOff).unwrap();

    thread::spawn(move || {
        for msg in rx {
            if let Event::Env(FakeEnvEvent::Done) = msg {
                tx_done.send(()).unwrap();
            } else if let Event::Env(FakeEnvEvent::Send { id, value }) = msg {
                tx_send.send((id, value)).unwrap();
            } else if let Event::Run(ExecutionEvent::TimerStart { .. }) = msg {
                tx_timer.send(true).unwrap();
            } else if let Event::Run(ExecutionEvent::TimerCancel { .. }) = msg {
                tx_timer.send(false).unwrap();
            } else {
                // Can be useful for debugging, but that's generally noise.
	            // println!("LOG: {:?}", msg)
            }
        }
    });

    let script_1 = Script {
        name: "Test script".to_owned(),
        rules: vec![
            Rule {
                conditions: vec![
                    Match {
                        source: vec![
                            ChannelSelector::new()
                        ],
                        kind: ChannelKind::LightOn,
                        range: Range::Eq(Value::OnOff(OnOff::On)),
                        duration: Some(Duration::from(chrono::Duration::seconds(10))),
                        phantom: PhantomData
                    }
                ],
                execute: vec![
                    Statement {
                        destination: vec![
                            ChannelSelector::new()
                        ],
                        value: data_off,
                        kind: ChannelKind::LightOn,
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
    let getter_id_1 = Id::<Channel>::new("Getter 1");
    let getter_id_2 = Id::<Channel>::new("Getter 2");
    let setter_id_1 = Id::<Channel>::new("Setter 1");

    sleep(&rx_done, &rx_send, &rx_timer);
	println!("[test_run_with_delay] * We can start executing a trivial rule.");
    exec.start(env.clone(), script_1, User::None, tx_run).unwrap();

    sleep(&rx_done, &rx_send, &rx_timer);
	println!("[test_run_with_delay] * Changing the structure of the network doesn't break the rule.");
    env.execute(Instruction::AddAdapters(vec![adapter_id_1.to_string()]));
    rx_done.recv().unwrap();

    env.execute(Instruction::AddServices(vec![
        Service::empty(&service_id_1, &adapter_id_1)
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::AddChannels(vec![
        Channel {
            kind: ChannelKind::LightOn,
            supports_fetch: true,
            supports_watch: true,
            .. Channel::empty(&getter_id_1, &service_id_1, &adapter_id_1)
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::AddChannels(vec![
        Channel {
            kind: ChannelKind::LightOn,
            supports_send: true,
            .. Channel::empty(&setter_id_1, &service_id_1, &adapter_id_1)
        }
    ]));
    rx_done.recv().unwrap();

    sleep(&rx_done, &rx_send, &rx_timer);
	println!("[test_run_with_delay] * Injecting the expected value is not sufficient to trigger the send.");
    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));

    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    sleep(&rx_done, &rx_send, &rx_timer);
	println!("[test_run_with_delay] * Waiting until the chrono fires triggers the send.");
    env.execute(Instruction::TriggerTimersUntil(TimeStamp::from(UTC::now() + ChronoDuration::seconds(15))));
    rx_done.recv().unwrap();

    let (id, value) = rx_send.recv().unwrap();
    assert_eq!(id, setter_id_1);
    assert_eq!(value, Value::OnOff(OnOff::Off));


    sleep(&rx_done, &rx_send, &rx_timer);
	println!("[test_run_with_delay] * Injecting an out-of-range value does not trigger the send, even if we wait.");
    env.execute(Instruction::ResetTimers);
    rx_done.recv().unwrap();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));

    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::TriggerTimersUntil(TimeStamp::from(UTC::now() + ChronoDuration::seconds(15))));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    sleep(&rx_done, &rx_send, &rx_timer);
	println!("[test_run_with_delay] * We can cancel the send by sending an out-of-range value before the delay.");
    env.execute(Instruction::ResetTimers);
    rx_done.recv().unwrap();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();
    assert_eq!(rx_timer.recv().unwrap(), true);

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    assert_eq!(rx_timer.recv().unwrap(), false);
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::TriggerTimersUntil(TimeStamp::from(UTC::now() + ChronoDuration::seconds(15))));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    sleep(&rx_done, &rx_send, &rx_timer);
	println!("[test_run_with_delay] * A getter removal cancels the send.");
    env.execute(Instruction::ResetTimers);
    rx_done.recv().unwrap();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::TriggerTimersUntil(TimeStamp::from(UTC::now() + ChronoDuration::seconds(2))));
    rx_done.recv().unwrap();

    env.execute(Instruction::RemoveChannels(vec![
        getter_id_1.clone()
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::TriggerTimersUntil(TimeStamp::from(UTC::now() + ChronoDuration::seconds(2))));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Err(APIError::TypeError(APITypeError {
            expected: Type::OnOff,
            got: Type::OpenClosed
        })))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::TriggerTimersUntil(TimeStamp::from(UTC::now() + ChronoDuration::seconds(15))));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    sleep(&rx_done, &rx_send, &rx_timer);
	println!("[test_run_with_delay] * With two devices, setting in-range value doesn't trigger the send immediately.");
    env.execute(Instruction::ResetTimers);
    rx_done.recv().unwrap();

    env.execute(Instruction::AddChannels(vec![
        Channel {
            kind: ChannelKind::LightOn,
            supports_fetch: true,
            supports_watch: true,
            .. Channel::empty(&getter_id_1, &service_id_1, &adapter_id_1)
        }
    ]));
    rx_done.recv().unwrap();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::Off))),
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::On)))
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::TriggerTimersUntil(TimeStamp::from(UTC::now() + ChronoDuration::seconds(5))));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    sleep(&rx_done, &rx_send, &rx_timer);
	println!("[test_run_with_delay] * With two devices, cancelling for one device doesn't cancel for all.");

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_2.clone(), Ok(Value::OnOff(OnOff::On))),
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::TriggerTimersUntil(TimeStamp::from(UTC::now() + ChronoDuration::seconds(1))));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::InjectGetterValues(vec![
        (getter_id_1.clone(), Ok(Value::OnOff(OnOff::Off))),
    ]));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    env.execute(Instruction::TriggerTimersUntil(TimeStamp::from(UTC::now() + ChronoDuration::seconds(5))));
    rx_done.recv().unwrap();
    rx_send.try_recv().unwrap_err();

    println!("[test_run_with_delay] * Test complete, cleaning up.");

    sleep(&rx_done, &rx_send, &rx_timer);
    rx_done.try_recv().unwrap_err();
    rx_send.try_recv().unwrap_err();

    println!("[test_run_with_delay] * Cleanup complete.");}

    println!("[test_run_with_delay] * Drop complete.");
    println!("[test_run_with_delay] END");
}
