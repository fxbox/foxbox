//! An adapter providing time-related services, such as the current
//! timestamp or the current time of day.

use adapt::*;
use utils::DispatchThread;

use foxbox_taxonomy::values::{Value, ValDuration, TimeStamp};
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::util::Id;

use std::boxed::FnBox;
use std::collections::{HashMap, HashSet};

use chrono;
use chrono::*;
use timer;

static ADAPTER_NAME: &'static str = "Clock adapter (built-in)";
static ADAPTER_VENDOR: &'static str = "team@link.mozilla.org";
static ADAPTER_VERSION: [u32;4] = [0, 0, 0, 0];

pub struct Clock {
    /// Timer used to dispatch `register_watch` requests.
    timer: timer::Timer,
    /// Thread used to execute callbacks.
    thread: DispatchThread,

    getter_timestamp_id: Id<Getter>,
    getter_time_of_day_id: Id<Getter>,
    service_clock_id: Id<ServiceId>,
}

/// A guard used to cancel watching for values.
struct Guard(Option<timer::Guard>);
impl WatchGuard for Guard {
}

impl Clock {
    pub fn id() -> Id<AdapterId> {
        Id::new("clock@link.mozilla.org".to_owned())
    }
    pub fn service_clock_id() -> Id<ServiceId> {
        Id::new("service:clock@link.mozilla.org".to_owned())
    }
    pub fn getter_timestamp_id() -> Id<Getter> {
        Id::new("getter:timestamp.clock@link.mozilla.org".to_owned())
    }
    pub fn getter_time_of_day_id() -> Id<Getter> {
        Id::new("getter:timeofday.clock@link.mozilla.org".to_owned())
    }
}
impl Adapter for Clock {
    fn id(&self) -> Id<AdapterId> {
        Self::id()
    }

    fn name(&self) -> &str {
        ADAPTER_NAME
    }

    fn vendor(&self) -> &str {
        ADAPTER_VENDOR
    }

    fn version(&self) -> &[u32;4] {
        &ADAPTER_VERSION
    }

    fn get_values(&self, set: Vec<Id<Getter>>) -> Vec<Result<Option<Value>, Error>> {
        set.iter().map(|id| {
            if *id == self.getter_timestamp_id {
                let date = TimeStamp::from_datetime(chrono::UTC::now());
                Ok(Some(Value::TimeStamp(date)))
            } else if *id == self.getter_time_of_day_id {
                use chrono::Timelike;
                let date = chrono::Local::now();
                let duration = chrono::Duration::seconds(date.num_seconds_from_midnight() as i64);
                Ok(Some(Value::Duration(ValDuration::new(duration))))
            } else {
                Err(Error::NoSuchGetter(id.clone()))
            }
        }).collect()
    }

    fn set_values(&self, mut values: Vec<(Id<Setter>, Value)>) -> Result<(), Error> {
        // This adapter doesn't support any setter.
        match values.pop() {
            None => Ok(()),
            Some((id, _)) => Err(Error::NoSuchSetter(id))
        }
    }

    fn register_watch(&self, id: Id<Getter>, threshold: Option<Value>, cb: Box<Fn(Value) + Send>) -> Result<Box<WatchGuard>, Error> {
        let threshold = match threshold {
            None => return Err(Error::GetterRequiresThresholdForWatching(id)),
            Some(threshold) => threshold
        };

        // Hack to workaround the fact that only `FnOnce` can move
        // values, but `Box<FnOnce>` is not implemented. We replace
        // the move with a call to `take()`.
        // This is part 1 of the hack.
        let mut cb = Some(cb);
        if id == self.getter_timestamp_id {
            let ts = try!(threshold.as_timestamp().map_err(Error::TypeError));
            // Use universal time.
            let now = chrono::offset::utc::UTC::now();
            if *ts.as_datetime() < now {
                // Too late to execute.
                return Ok(Box::new(Guard(None)));
            }

            let thread = self.thread.clone();
            let guard = self.timer.schedule_with_date(*ts.as_datetime(), move || {
                // Hack to workaround the fact that only `FnOnce` can move
                // values, but `Box<FnOnce>` is not implemented. We replace
                // the move with a call to `take()`.
                // This is part 2 of the hack.
                let mut cb = Some(cb.take().unwrap());
                let cb2 = move || {
                    let value = Value::TimeStamp(TimeStamp::from_datetime(chrono::UTC::now()));
                    // Hack to workaround the fact that only `FnOnce` can move
                    // values, but `Box<FnOnce>` is not implemented. We replace
                    // the move with a call to `take()`.
                    // This is part 3 of the hack.
                    let cb = cb.take().unwrap();
                    cb(value);
                };
                thread.dispatch(cb2);
            });
            Ok(Box::new(Guard(Some(guard))))
        } else if id == self.getter_time_of_day_id {
            let time_of_day = try!(threshold.as_duration().map_err(Error::TypeError)).as_duration();
            let thread = self.thread.clone();

            // Use local time.
            let ts : chrono::DateTime<_> = match chrono::Local::today().and_time(NaiveTime::from_hms(0, 0, 0) + *time_of_day) {
                None => return Err(Error::InvalidValue),
                Some(ts) => ts.with_timezone(&UTC)
            };

            let guard = self.timer.schedule(ts, Some(Duration::days(1)), move || {
                // Hack to workaround the fact that only `FnOnce` can move
                // values, but `Box<FnOnce>` is not implemented. We replace
                // the move with a call to `take()`.
                // This is part 2 of the hack.
                let mut cb = Some(cb.take().unwrap());
                let cb2 = move || {
                    let value = Value::TimeStamp(TimeStamp::from_datetime(chrono::UTC::now()));
                    // Hack to workaround the fact that only `FnOnce` can move
                    // values, but `Box<FnOnce>` is not implemented. We replace
                    // the move with a call to `take()`.
                    // This is part 3 of the hack.
                    let cb = cb.take().unwrap();
                    cb(value);
                };
                thread.dispatch(cb2);
            });
            Ok(Box::new(Guard(Some(guard))))
        } else {
            Err(Error::GetterDoesNotSupportWatching(id))
        }
    }
}

impl Clock {
    pub fn init(adapt: &AdapterControl, cb: Box<FnBox(Result<(), Error>) + Send> ) {
        // Setup a thread to execute callbacks.
        let thread = DispatchThread::new();
        let getter_timestamp_id = Clock::getter_timestamp_id();
        let getter_time_of_day_id = Clock::getter_time_of_day_id();
        let service_clock_id = Clock::service_clock_id();
        let clock = Clock {
            thread: thread,
            timer: timer::Timer::new(),
            getter_timestamp_id: getter_timestamp_id.clone(),
            getter_time_of_day_id: getter_time_of_day_id.clone(),
            service_clock_id: service_clock_id.clone(),
        };
        adapt.add_adapter(clock, vec![Service {
            adapter: Clock::id(),
            tags: HashSet::new(),
            id: service_clock_id.clone(),
            setters: HashMap::new( /* No setters */ ),
            getters: vec![
                // Time of day
                (getter_time_of_day_id.clone(), Channel {
                    tags: HashSet::new(),
                    id: getter_time_of_day_id.clone(),
                    last_seen: None,
                    service: service_clock_id.clone(),
                    mechanism: Getter {
                        kind: ChannelKind::CurrentTimeOfDay,
                        poll: Some(ValDuration::new(chrono::Duration::seconds(1))),
                        trigger: None,
                        watch: true,
                        updated: None
                    }
                }),

                // Current time
                (getter_timestamp_id.clone(), Channel {
                    tags: HashSet::new(),
                    id: getter_timestamp_id.clone(),
                    last_seen: None,
                    service: service_clock_id.clone(),
                    mechanism: Getter {
                        kind: ChannelKind::CurrentTime,
                        poll: Some(ValDuration::new(chrono::Duration::seconds(1))),
                        trigger: None,
                        watch: true,
                        updated: None
                    }
                })].iter().cloned().collect(),
        }],
        cb);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use adapt::*;

    use foxbox_taxonomy::util::Id;
    use foxbox_taxonomy::services::*;

    use std::collections::HashSet;
    use std::sync::mpsc::channel;
    use std::boxed::FnBox;

    fn make_sync<F, T>(cb: F) -> T
        where F: FnOnce(Box<FnBox(T) + Send>),
              T : 'static + Send {
        let (tx, rx) = channel();
        let cb2 = move |result: T| {
            tx.send(result).unwrap();
        };
        cb(Box::new(cb2));
        rx.recv().unwrap()
    }
    #[test]
    fn test_add_remove_clock() {
        let control = AdapterControl::new();
        println!("Initializing the clock the first time should work");
        assert!(make_sync(|done| Clock::init(&control, done)).is_ok());

        println!("Initializing the clock the first time should fail because of a duplicate adapter");
        match make_sync(|done| Clock::init(&control, done)) {
            Err(Error::DuplicateAdapter(_)) => {},
            other => panic!("Didn't expect result {:?}", other)
        }

        println!("Try removing twice");
        make_sync(|done| control.remove_adapter(&Clock::id(), done)).unwrap();

        match make_sync(|done| control.remove_adapter(&Clock::id(), done)) {
            Err(Error::NoSuchAdapter(_)) => {},
            other => panic!("Didn't expect result {:?}", other)
        }

        println!("Initializing the clock should work again");
        assert!(make_sync(|done| Clock::init(&control, done)).is_ok());
    }

    #[test]
    fn test_add_remove_services() {
        let control = AdapterControl::new();
        assert!(make_sync(|done| Clock::init(&control, done)).is_ok());

        let no_such_adapter_id : Id<AdapterId> = Id::new("no such adapter".to_owned());
        let example_service_id : Id<ServiceId> = Id::new("example service".to_owned());
        let example_service_2_id : Id<ServiceId> = Id::new("example service 2".to_owned());
        let no_such_service_id : Id<ServiceId> = Id::new("no such service".to_owned());
        let example_getter_id : Id<Getter> = Id::new("example getter".to_owned());

        println!("Attempting to add a service to a non-existing adapter");
        match make_sync(|done| control.add_service(&no_such_adapter_id,
            Service::empty(example_service_id.clone(), Clock::id()), done)) {
                Err(Error::NoSuchAdapter(_)) => {},
                other => panic!("Didn't expect result {:?}", other)
            }

        println!("Attempting to overwrite a service that was already installed by the adapter");
        match make_sync(|done| control.add_service(&Clock::id(),
            Service::empty(Clock::service_clock_id(), Clock::id()), done)) {
                Err(Error::DuplicateService(_)) => {},
                other => panic!("Didn't expect result {:?}", other)
            }

        println!("Attempting to add a service that would overwrite a getter");
        match make_sync(|done| control.add_service(&Clock::id(),
            Service {
                getters: vec![(
                    Clock::getter_timestamp_id(),
                    Channel {
                        id: Clock::getter_timestamp_id(),
                        tags: HashSet::new(),
                        last_seen: None,
                        service: Clock::service_clock_id(),
                        mechanism: Getter {
                            kind: ChannelKind::Ready,
                            poll: None,
                            trigger: None,
                            watch: false,
                            updated: None,
                        }
                    }
                )].iter().cloned().collect(),
                .. Service::empty(no_such_service_id.clone(), Clock::id())
            }, done)) {
                Err(Error::DuplicateGetter(_)) => {},
                other => panic!("Didn't expect result {:?}", other)
            }

            println!("Attempting to add a service that with a fresh getter");
            match make_sync(|done| control.add_service(&Clock::id(),
                Service {
                    getters: vec![(
                        example_getter_id.clone(),
                        Channel {
                            id: example_getter_id.clone(),
                            tags: HashSet::new(),
                            last_seen: None,
                            service: Clock::service_clock_id(),
                            mechanism: Getter {
                                kind: ChannelKind::Ready,
                                poll: None,
                                trigger: None,
                                watch: false,
                                updated: None,
                            }
                        }
                    )].iter().cloned().collect(),
                    .. Service::empty(example_service_id.clone(), Clock::id())
                }, done)) {
                    Ok(_) => {},
                    other => panic!("Didn't expect result {:?}", other)
                }

                println!("Attempting to add a service that also offers the no-longer-fresh getter.");
                match make_sync(|done| control.add_service(&Clock::id(),
                    Service {
                        getters: vec![(
                            Clock::getter_timestamp_id(),
                            Channel {
                                id: example_getter_id.clone(),
                                tags: HashSet::new(),
                                last_seen: None,
                                service: Clock::service_clock_id(),
                                mechanism: Getter {
                                    kind: ChannelKind::Ready,
                                    poll: None,
                                    trigger: None,
                                    watch: false,
                                    updated: None,
                                }
                            }
                        )].iter().cloned().collect(),
                        .. Service::empty(example_service_2_id.clone(), Clock::id())
                    }, done)) {
                        Err(Error::DuplicateGetter(_)) => {},
                        other => panic!("Didn't expect result {:?}", other)
                    }


                println!("Now, removing the fresh service");
                make_sync(|done| control.remove_service(&Clock::id(), &example_service_id, done)).unwrap();

                println!("Can we remove it twice?");
                match make_sync(|done| control.remove_service(&Clock::id(), &example_service_id, done)) {
                    Err(Error::NoSuchService(_)) => {},
                    other => panic!("Didn't expect result {:?}", other)
                }

                println!("Remove a service that hasn't been added");
                match make_sync(|done| control.remove_service(&Clock::id(), &no_such_service_id, done)) {
                    Err(Error::NoSuchService(_)) => {},
                    other => panic!("Didn't expect result {:?}", other)
                }

                println!("Now, removing a built-in service");
                make_sync(|done| control.remove_service(&Clock::id(), &Clock::service_clock_id(), done)).unwrap();

                println!("Checking that we can still remove the adapter");
                make_sync(|done| control.remove_adapter(&Clock::id(), done)).unwrap();

                println!("And add it back, despite our manipulations");
                make_sync(|done| Clock::init(&control, done)).unwrap();
    }

}