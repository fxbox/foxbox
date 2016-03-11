//! An adapter providing time-related services, such as the current
//! timestamp or the current time of day.

use foxbox_adapters::adapter::*;
use foxbox_taxonomy::api::{ AdapterError, ResultMap };
use foxbox_taxonomy::values::{ Range, TimeStamp, Type, ValDuration, Value };
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::util::Id;

use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use std::thread;
use std::sync::mpsc:: { channel, Sender };

use chrono;
use chrono::*;
use timer;

static ADAPTER_NAME: &'static str = "Clock adapter (built-in)";
static ADAPTER_VENDOR: &'static str = "team@link.mozilla.org";
static ADAPTER_VERSION: [u32;4] = [0, 0, 0, 0];

#[derive(Clone)]
enum Op {
    Enter(Id<Getter>, Value),
    Exit(Id<Getter>, Value),
}

enum Movement { Enter, Exit }

pub struct Clock {
    /// Timer used to dispatch `register_watch` requests.
    timer: timer::Timer,
    getter_timestamp_id: Id<Getter>,
    getter_time_of_day_id: Id<Getter>,

    service_clock_id: Id<ServiceId>,
}

/// A guard used to cancel watching for values.
struct Guard(Vec<timer::Guard>);
impl AdapterWatchGuard for Guard {
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

    fn fetch_values(&self, set: Vec<Id<Getter>>) -> ResultMap<Id<Getter>, Option<Value>, AdapterError> {
        set.iter().map(|id| {
            if *id == self.getter_timestamp_id {
                let date = TimeStamp::from_datetime(chrono::UTC::now());
                (id.clone(), Ok(Some(Value::TimeStamp(date))))
            } else if *id == self.getter_time_of_day_id {
                use chrono::Timelike;
                let date = chrono::Local::now();
                let duration = chrono::Duration::seconds(date.num_seconds_from_midnight() as i64);
                (id.clone(), Ok(Some(Value::Duration(ValDuration::new(duration)))))
            } else {
                (id.clone(), Err(AdapterError::NoSuchGetter(id.clone())))
            }
        }).collect()
    }

    fn send_values(&self, values: Vec<(Id<Setter>, Value)>) -> ResultMap<Id<Setter>, (), AdapterError> {
        values.iter()
            .map(|&(ref id, _)| {
                (id.clone(), Err(AdapterError::NoSuchSetter(id.clone())))
            })
            .collect()
    }

    fn register_watch(&self, mut watch: Vec<(Id<Getter>, Option<Range>)>,
        cb: Box<Fn(WatchEvent) + Send>) ->
            ResultMap<Id<Getter>, Box<AdapterWatchGuard>, AdapterError>
    {
        let (tx, rx) = channel();
        thread::spawn(move || {
            let cb = cb;
            for msg in rx {
                match msg {
                    Op::Enter(id, value) => {
                        cb(WatchEvent::Enter {
                            id: id,
                            value: value
                        })
                    },
                    Op::Exit(id, value) => {
                        cb(WatchEvent::Exit {
                            id: id,
                            value: value
                        })
                    },
                }
            }
        });
        watch.drain(..).map(|(id, filter)| {
            (id.clone(), match filter {
                None => Err(AdapterError::GetterRequiresThresholdForWatching(id)),
                Some(range) => self.aux_register_watch(&id, range, tx.clone())
            })
        }).collect()
    }
}

impl Clock {
    fn aux_register_watch(&self, id: &Id<Getter>, range: Range, tx: Sender<Op>)
        -> Result<Box<AdapterWatchGuard>, AdapterError>
    {
        match () {
            _ if *id == self.getter_time_of_day_id => self.aux_register_watch_timeofday(id, range, tx),
            _ if *id == self.getter_timestamp_id => self.aux_register_watch_timestamp(id, range, tx),
            _ => Err(AdapterError::GetterDoesNotSupportWatching(id.clone()))
        }
    }

    fn aux_register_watch_timeofday(&self, id: &Id<Getter>, range: Range, tx: Sender<Op>)
        -> Result<Box<AdapterWatchGuard>, AdapterError>
    {
        use foxbox_taxonomy::values::Range::*;

        // Sanity checks
        let typ = try!(range.get_type().map_err(AdapterError::TypeError));
        try!(Type::Duration.ensure_eq(&typ).map_err(AdapterError::TypeError));

        // Now determine when to call the trigger. Repeat duration is always one day.
        let mut thresholds = match range {
            Leq (ref val) => {
                // Equivalent to BetweenEq { min: 0am, max: val }
                let ts = *try!(val.as_duration().map_err(AdapterError::TypeError))
                    .as_duration();
                vec![(Movement::Enter, Duration::seconds(0)), (Movement::Exit, ts)]
            }
            Geq (ref val) => {
                // Equivalent to BetweenEq { min: val, max: 0am }
                let ts = *try!(val.as_duration().map_err(AdapterError::TypeError))
                    .as_duration();
                vec![(Movement::Enter, ts), (Movement::Exit, Duration::days(1))]
            }
            BetweenEq { ref min, ref max } => {
                let ts_min = *try!(min.as_duration().map_err(AdapterError::TypeError))
                    .as_duration();
                let ts_max = *try!(max.as_duration().map_err(AdapterError::TypeError))
                    .as_duration();
                vec![(Movement::Enter, ts_min), (Movement::Exit, ts_max)]
            }
            OutOfStrict { ref min, ref max } => {
                // Equivalent to BetweenEq {min: 0am, max: min} and BetweenEq {min: max, max: 0am}
                let ts_min = *try!(min.as_duration().map_err(AdapterError::TypeError))
                    .as_duration();
                let ts_max = *try!(max.as_duration().map_err(AdapterError::TypeError))
                    .as_duration();
                vec![(Movement::Exit, ts_min), (Movement::Enter, ts_max)]
            }
            Eq (ref val) => {
                let ts = *try!(val.as_duration().map_err(AdapterError::TypeError))
                    .as_duration();
                vec![(Movement::Enter, ts.clone()), (Movement::Exit, ts)]
            }
        };

        // Determine when the next timers needs to launch.
        let now = chrono::Local::now();
        let guards : Vec<timer::Guard> = thresholds.drain(..).filter_map(|(movement, threshold)| {
            let date = match Self::get_next_date(&now, threshold) {
                Err(_) => return None,
                Ok(date) => date,
            };
            let id = id.clone();
            let tx = tx.clone();
            let guard = self.timer.schedule(date, Some(Duration::days(1)), move || {
                let naive_time = chrono::Local::now().time();
                let duration = Duration::hours(naive_time.hour() as i64)
                    + Duration::minutes(naive_time.minute() as i64)
                    + Duration::seconds(naive_time.second() as i64);

                let event = match movement {
                    Movement::Enter => Op::Enter(id.clone(),
                        Value::Duration(ValDuration::new(duration))),
                    Movement::Exit => Op::Exit(id.clone(),
                        Value::Duration(ValDuration::new(duration))),
                };
                let _ = tx.send(event);
            });
            Some(guard)
        }).collect();
        Ok(Box::new(Guard(guards)))
    }

    fn get_next_date(now: &DateTime<Local>, time_of_day: Duration)
        -> Result<DateTime<Local>, AdapterError>
    {
        match chrono::Local::today().and_time(NaiveTime::from_hms(0, 0, 0) + time_of_day) {
            None => Err(AdapterError::InvalidValue),
            Some(date) => {
                if date >= *now  {
                    Ok(date)
                } else {
                    // Otherwise, shift to tomorrow.
                    match date.checked_add(Duration::days(1)) {
                        None => Err(AdapterError::InvalidValue),
                        Some(date) => Ok(date)
                    }
                }
            }
        }
    }

    fn aux_register_watch_timestamp(&self, id: &Id<Getter>, range: Range, tx: Sender<Op>)
        -> Result<Box<AdapterWatchGuard>, AdapterError>
    {
        use foxbox_taxonomy::values::Range::*;

        // Sanity checks
        let typ = try!(range.get_type().map_err(AdapterError::TypeError));
        try!(Type::TimeStamp.ensure_eq(&typ).map_err(AdapterError::TypeError));

        // Now determine when/if to call the trigger.
        let mut thresholds = match range {
            Leq (_) => {
                // This variant doesn't make sense.
                return Ok(Box::new(Guard(vec![])))
            }
            Geq (ref val) | Eq (ref val) => {
                let ts = *try!(val.as_timestamp().map_err(AdapterError::TypeError))
                    .as_datetime();
                vec![(Movement::Enter, ts)]
            }
            OutOfStrict { ref min, ref max } => {
                let ts_min = *try!(min.as_timestamp().map_err(AdapterError::TypeError))
                    .as_datetime();
                let ts_max = *try!(max.as_timestamp().map_err(AdapterError::TypeError))
                    .as_datetime();
                vec![(Movement::Exit, ts_min), (Movement::Enter, ts_max)]
            }
            BetweenEq { ref min, ref max } => {
                let ts_min = *try!(min.as_timestamp().map_err(AdapterError::TypeError))
                    .as_datetime();
                let ts_max = *try!(max.as_timestamp().map_err(AdapterError::TypeError))
                    .as_datetime();
                vec![(Movement::Enter, ts_min), (Movement::Exit, ts_max)]
            }
        };

        // Determine when/if the next timers needs to launch.
        let now = chrono::UTC::now();
        let guards : Vec<timer::Guard> = thresholds.drain(..).filter_map(|(movement, date)| {
            if date < now {
                return None
            }
            let id = id.clone();
            let tx = tx.clone();
            let guard = self.timer.schedule_with_date(date, move || {
                let naive_time = chrono::Local::now().time();
                let duration = Duration::hours(naive_time.hour() as i64)
                    + Duration::minutes(naive_time.minute() as i64)
                    + Duration::seconds(naive_time.second() as i64);

                let event = match movement {
                    Movement::Enter => Op::Enter(id.clone(),
                        Value::Duration(ValDuration::new(duration))),
                    Movement::Exit => Op::Exit(id.clone(),
                        Value::Duration(ValDuration::new(duration))),
                };
                let _ = tx.send(event);
            });
            Some(guard)
        }).collect();
        Ok(Box::new(Guard(guards)))
    }
}

impl Clock {
    pub fn init<T>(adapt: &Arc<T>) -> Result<(), AdapterError>
        where T: AdapterManagerHandle
    {
        let getter_timestamp_id = Clock::getter_timestamp_id();
        let getter_time_of_day_id = Clock::getter_time_of_day_id();
        let service_clock_id = Clock::service_clock_id();
        let clock = Box::new(Clock {
            timer: timer::Timer::new(),
            getter_timestamp_id: getter_timestamp_id.clone(),
            getter_time_of_day_id: getter_time_of_day_id.clone(),
            service_clock_id: service_clock_id.clone(),
        });
        adapt.add_adapter(clock, vec![Service {
            adapter: Clock::id(),
            tags: HashSet::new(),
            id: service_clock_id.clone(),
            setters: HashMap::new( /* No setters */ ),
            getters: vec![
                // Time of day
                (getter_time_of_day_id.clone(), Channel {
                    tags: HashSet::new(),
                    adapter: Clock::id(),
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
                    adapter: Clock::id(),
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
        }])
    }
}
