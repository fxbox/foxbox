//! An adapter providing access to the Thinkerbell rules engine.

use foxbox_taxonomy::api::{ Error, InternalError, User };
use foxbox_taxonomy::manager::*;
use foxbox_taxonomy::services::{ Setter, Getter, AdapterId, ServiceId, Service, Channel, ChannelKind };
use foxbox_taxonomy::util::Id;
use foxbox_taxonomy::values::{ Duration, Type, Value, TypeError, OnOff };

use foxbox_thinkerbell::compile::ExecutableDevEnv;
use foxbox_thinkerbell::manager::{ ScriptManager, ScriptId, Error as ScriptManagerError };
use foxbox_thinkerbell::run::ExecutionEvent;

use timer;
use transformable_channels::mpsc::*;

use std::collections::{ HashMap, HashSet };
use std::fmt;
use std::path::Path;
use std::sync::{ Arc, Mutex };
use std::thread;

static ADAPTER_NAME: &'static str = "Thinkerbell adapter (built-in)";
static ADAPTER_VENDOR: &'static str = "team@link.mozilla.org";
static ADAPTER_VERSION: [u32;4] = [0, 0, 0, 0];

/// `ThinkerbellAdapter` hooks up the rules engine (if this, then that) as an adapter.
///
/// Each "rule", or "script", is a JSON-serialized structure according to Thinkerbell conventions.
///
/// This adapter exposes a root service, with one `AddThinkerbellRule` setter (to add a new rule).
/// Each rule that has been added is exposed as its own service, with the following getters/setters:
/// - Set Enabled (setter) -- toggles whether or not the script is enabled
/// - Get Enabled (getter) -- returns whether or not the script is enabled
/// - Remove (setter) -- removes the script
///
/// This adapter performs most actions by delegating channel messages to its main thread.
#[derive(Clone)]
pub struct ThinkerbellAdapter {

    /// The sending end of the channel for sending messages to `ThinkerbellAdapter`'s main loop.
    tx: Arc<Mutex<RawSender<ThinkAction>>>,

    /// A reference to the AdapterManager.
    adapter_manager: Arc<AdapterManager>,

    /// The ID of this adapter (permanently fixed)
    adapter_id: Id<AdapterId>,

    /// The ID of the root service's "Add Rule" setter.
    setter_add_rule_id: Id<Setter>,
}

/// Thinkerbell requires an execution environment following this API.
#[derive(Clone)]
struct ThinkerbellExecutionEnv {
    adapter_manager: Arc<AdapterManager>,

    // FIXME: Timer's not clonable, so we should only use one, right? Does this have to be mutexed?
    timer: Arc<Mutex<timer::Timer>>
}
impl fmt::Debug for ThinkerbellExecutionEnv {
    fn fmt(&self, _: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        Ok(())
    }
}


impl ExecutableDevEnv for ThinkerbellExecutionEnv {
    // We don't support watches, so we don't care about the type of WatchGuard.
    type WatchGuard = WatchGuard;
    type API = AdapterManager;

    fn api(&self) -> &Self::API {
        &*self.adapter_manager
    }

    type TimerGuard = timer::Guard;
    fn start_timer(&self, duration: Duration, sender: Box<ExtSender<()>>) -> Self::TimerGuard {
        let timer = self.timer.lock().unwrap(); // FIXME: There's no way to report this error...
        timer.schedule_with_delay(duration.into(), move || {
            let _ = sender.send(());
        })
    }
}

/// Convert a `ScriptManagerError` into an API Error.
/// We can't implement From<T> because `ScriptManagerError` is in a different crate.
fn sm_error(e: ScriptManagerError) -> Error {
    Error::InternalError(InternalError::GenericError(format!("{:?}", e)))
}

impl Adapter for ThinkerbellAdapter {
    fn id(&self) -> Id<AdapterId> {
        self.adapter_id.clone()
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

    fn fetch_values(&self, set: Vec<Id<Getter>>, _: User) -> ResultMap<Id<Getter>, Option<Value>, Error> {
        set.iter().map(|id| {
            let (tx, rx) = channel();
            let _ = self.tx.lock().unwrap().send(ThinkAction::RespondToGetter(tx, id.clone()));
            match rx.recv() {
                Ok(result) => (id.clone(), result),
                // If an error occurs, the channel/thread died!
                Err(recv_err) => (id.clone(), Err(Error::InternalError(
                        InternalError::GenericError(format!("{:?}", recv_err))))),
            }
        }).collect()
    }

    fn send_values(&self, values: HashMap<Id<Setter>, Value>, user: User) -> ResultMap<Id<Setter>, (), Error> {
        values.iter()
            .map(|(id, value)| {
                let (tx, rx) = channel();
                let _ = self.tx.lock().unwrap().send(ThinkAction::RespondToSetter(tx, id.clone(), value.clone(), user.clone()));
                match rx.recv() {
                    Ok(result) => (id.clone(), result),
                    // If an error occurs, the channel died!
                    Err(recv_err) => (id.clone(), Err(Error::InternalError(
                            InternalError::GenericError(format!("{:?}", recv_err))))),
                }
            })
            .collect()
    }

    fn register_watch(&self, mut watch: Vec<WatchTarget>) -> WatchResult
    {
        watch.drain(..).map(|(id, _, _)| {
            (id.clone(), Err(Error::GetterDoesNotSupportWatching(id)))
        }).collect()
    }
}

/// `ThinkerbellAdapter`'s main loop handles messages of these types.
enum ThinkAction {
    AddRuleService(Id<ScriptId>),
    RemoveRuleService(Id<ScriptId>),
    RespondToGetter(RawSender<Result<Option<Value>, Error>>, Id<Getter>),
    RespondToSetter(RawSender<Result<(), Error>>, Id<Setter>, Value, User),
}

/// An internal data structure to track getters and setters.
struct ThinkerbellRule {
    script_id: Id<ScriptId>,
    service_id: Id<ServiceId>,
    getter_source_id: Id<Getter>,
    getter_is_enabled_id: Id<Getter>,
    setter_is_enabled_id: Id<Setter>,
    setter_remove_id: Id<Setter>,
}

impl ThinkerbellAdapter {

    #[allow(cyclomatic_complexity)]
    fn main(
        &self,
        rx: Receiver<ThinkAction>,
        mut script_manager: ScriptManager<ThinkerbellExecutionEnv, RawSender<(Id<ScriptId>, ExecutionEvent)>>
    ) {
        // Store an in-memory list of all of the rules (their getters, setters, etc.).
        // We need to track these to respond to getter/setter requests.
        let mut rules: Vec<ThinkerbellRule> = Vec::new();

        'recv: for action in rx {
            match action {
                // After a script has been started, start a Service for that script.
                // The script has already been started with ScriptManager at this point;
                // we're just adding the adapter's services now.
                ThinkAction::AddRuleService(script_id) => {
                    // If the rule already existed (i.e. we're overwriting the original source),
                    // we don't need to re-add a Service. This is safe, because a Service doesn't
                    // know or care about the contents of the rule, just the ID.
                    for ref rule in &rules {
                        if rule.script_id == script_id {
                            info!("[thinkerbell@link.mozilla.org] No need to create a new service for this rule; ID '{}' already exists.", &script_id);
                            continue 'recv;
                        }
                    }
                    match self.add_rule_service(script_id) {
                        Ok(rule) => {
                            rules.push(rule);
                        },
                        Err(e) => {
                            error!("[thinkerbell@link.mozilla.org] Unable to add Thinkerbell Rule Service: {:?}", e);
                        }
                    };
                },
                // After a script has been stopped, remove the Service for that script.
                // The script has already been removed from ScriptManager at this point;
                // we're just updating the Service-level bookkeeping.
                ThinkAction::RemoveRuleService(script_id) => {
                    if let Some(position) = rules.iter().position(|ref r| r.script_id == script_id) {
                        let rule = rules.remove(position);
                        match self.remove_rule_service(&rule) {
                            Ok(_) => {},
                            Err(e) => {
                                error!("[thinkerbell@link.mozilla.org] Unable to remove Thinkerbell Rule Service: {:?}", e)
                            }
                        }
                    }
                },
                // Respond to a pending Getter request.
                ThinkAction::RespondToGetter(tx, getter_id) => {
                    for ref rule in &rules {
                        if getter_id == rule.getter_is_enabled_id {
                            let is_enabled = script_manager.is_enabled(&rule.script_id);
                            let _ = tx.send(Ok(Some(Value::OnOff(if is_enabled { OnOff::On } else { OnOff::Off }))));
                            continue 'recv;
                        } else if getter_id == rule.getter_source_id {
                            match script_manager.get_source_and_owner(&rule.script_id) {
                                Ok((source, _)) => {
                                    let _ = tx.send(Ok(Some(Value::String(Arc::new(source.to_owned())))));
                                },
                                Err(e) => {
                                    let _ = tx.send(Err(sm_error(e)));
                                }
                            };
                            continue 'recv;
                        }
                    }
                    let _ = tx.send(Err(Error::InternalError(InternalError::NoSuchGetter(getter_id.clone()))));
                },
                // Respond to a pending Setter request.
                ThinkAction::RespondToSetter(tx, setter_id, value, user) => {
                    // Add a new rule (with the given JSON source).
                    if setter_id == self.setter_add_rule_id {
                        match value {
                            Value::ThinkerbellRule(ref rule_source) => {
                                let script_id = Id::new(&rule_source.name);
                                match script_manager.put(&script_id, &rule_source.source, &user) {
                                    Err(err) => {let _ = tx.send(Err(sm_error(err))) ;}
                                    Ok(ok) => {
                                        let _ = tx.send(Ok(ok));
                                        let _ = self.tx.lock().unwrap().send(ThinkAction::AddRuleService(script_id.clone()));
                                    }
                                }
                            },
                            _ => {
                                let _ = tx.send(Err(Error::TypeError(TypeError {
                                    expected: Type::ThinkerbellRule,
                                    got: value.get_type()
                                })));
                            }
                        }
                    } else {
                        // The rest of the rules are script/rule-specific.
                        // NOTE: This linear search is not ideal, but tracking getters/setters in maps
                        // would be far more complex until we have a simpler way to track state within
                        // getter/setter API requests. In any case, this loop should be plenty fast for now.
                        for ref rule in &rules {
                            if setter_id == rule.setter_is_enabled_id {
                                match value {
                                    Value::OnOff(OnOff::On) => {
                                        let _ = tx.send(script_manager.set_enabled(&rule.script_id, true).map_err(sm_error));
                                    },
                                    Value::OnOff(OnOff::Off) => {
                                        let _ = tx.send(script_manager.set_enabled(&rule.script_id, false).map_err(sm_error));
                                    },
                                    _ => {
                                        let _ = tx.send(Err(Error::TypeError(TypeError {
                                            expected: Type::OnOff,
                                            got: value.get_type()
                                        })));
                                    },
                                }
                                continue 'recv;
                            } else if setter_id == rule.setter_remove_id {
                                let _ = tx.send(script_manager.remove(&rule.script_id).map_err(sm_error));
                                let _ = self.tx.lock().unwrap().send(ThinkAction::RemoveRuleService(rule.script_id.clone()));
                                continue 'recv;
                            }
                        }
                        // If we got here, no setters matched.
                        let _ = tx.send(Err(Error::InternalError(InternalError::NoSuchSetter(setter_id.clone()))));
                    }
                }
            }
        }
    }

    /// Add a new service for a script. (This does not start this script, this just adds a Service.)
    fn add_rule_service(&self, script_id: Id<ScriptId>) -> Result<ThinkerbellRule, Error> {
        let service_id = Id::new(&format!("thinkerbell/{}", script_id.as_atom()));

        let rule = ThinkerbellRule {
            script_id: script_id.clone(),
            service_id: service_id.clone(),
            getter_source_id: Id::new(&format!("{}/source", service_id.as_atom())),
            getter_is_enabled_id: Id::new(&format!("{}/get_enabled", service_id.as_atom())),
            setter_is_enabled_id: Id::new(&format!("{}/set_enabled", service_id.as_atom())),
            setter_remove_id: Id::new(&format!("{}/remove", service_id.as_atom())),
        };

        try!(self.adapter_manager.add_service(Service::empty(service_id.clone(), self.adapter_id.clone())));

        try!(self.adapter_manager.add_getter(Channel {
            id: rule.getter_is_enabled_id.clone(),
            service: service_id.clone(),
            adapter: self.adapter_id.clone(),
            last_seen: None,
            tags: HashSet::new(),
            mechanism: Getter {
                kind: ChannelKind::ThinkerbellRuleOn,
                updated: None
            },
        }));

        // Add getter for script source
        try!(self.adapter_manager.add_getter(Channel {
            id: rule.getter_source_id.clone(),
            service: service_id.clone(),
            adapter: self.adapter_id.clone(),
            last_seen: None,
            tags: HashSet::new(),
            mechanism: Getter {
                kind: ChannelKind::ThinkerbellRuleSource,
                updated: None
            },
        }));

        // Add setter for set_enabled
        try!(self.adapter_manager.add_setter(Channel {
            id: rule.setter_is_enabled_id.clone(),
            service: service_id.clone(),
            adapter: self.adapter_id.clone(),
            last_seen: None,
            tags: HashSet::new(),
            mechanism: Setter {
                kind: ChannelKind::ThinkerbellRuleOn,
                updated: None
            },
        }));

        // Add setter for removing this rule.
        try!(self.adapter_manager.add_setter(Channel {
            id: rule.setter_remove_id.clone(),
            service: service_id.clone(),
            adapter: self.adapter_id.clone(),
            last_seen: None,
            tags: HashSet::new(),
            mechanism: Setter {
                kind: ChannelKind::RemoveThinkerbellRule,
                updated: None
            },
        }));

        info!("[thinkerbell@link.mozilla.org] Added Thinkerbell Rule for '{}'", &script_id.to_string());

        Ok(rule)
    }

    /// Remove an already-added Service (this does not stop the script).
    fn remove_rule_service(&self, rule: &ThinkerbellRule) -> Result<(), Error> {
        info!("[thinkerbell@link.mozilla.org] Removed Thinkerbell Rule for '{}'", &rule.script_id.to_string());
        self.adapter_manager.remove_service(&rule.service_id)
    }

    /// Everything is initialized here, but the real work happens in the main() loop.
    pub fn init(manager: &Arc<AdapterManager>, scripts_path: &str) -> Result<(), Error> {
        let adapter_id = Id::new("thinkerbell@link.mozilla.org");
        let setter_add_rule_id = Id::new("thinkerbell-add-rule");
        let root_service_id = Id::new("thinkerbell-root-service");

        // Prepare the script execution environment and load existing scripts.
        let (tx_env, rx_env) = channel();
        let env = ThinkerbellExecutionEnv {
            adapter_manager: manager.clone(),
            timer: Arc::new(Mutex::new(timer::Timer::new()))
        };

        let mut script_manager = try!(
            ScriptManager::new(env, Path::new(scripts_path), Box::new(tx_env)).map_err(sm_error));

        let result_map = try!(script_manager.load().map_err(sm_error));

        let (tx, rx) = channel();

        for script_id in result_map.keys() {
            let _ = tx.send(ThinkAction::AddRuleService(script_id.clone()));
        }

        let adapter = ThinkerbellAdapter {
            tx: Arc::new(Mutex::new(tx)),
            adapter_manager: manager.clone(),
            adapter_id: adapter_id.clone(),
            setter_add_rule_id: setter_add_rule_id.clone(),
        };

        // Add the adapter and the root service (the one that exposes `AddThinkerbellRule` for adding new rules).
        try!(manager.add_adapter(Arc::new(adapter.clone())));
        try!(manager.add_service(Service::empty(root_service_id.clone(), adapter_id.clone())));
        try!(manager.add_setter(Channel {
            id: setter_add_rule_id.clone(),
            service: root_service_id.clone(),
            adapter: adapter_id.clone(),
            last_seen: None,
            tags: HashSet::new(),
            mechanism: Setter {
                kind: ChannelKind::AddThinkerbellRule,
                updated: None
            },
        }));

        thread::spawn(move || {
            info!("[thinkerbell@link.mozilla.org] Started Thinkerbell main thread.");
            adapter.main(rx, script_manager)
        });

        // FIXME: We need to consume the events from the execution environment to prevent the
        // queue from growing unboundedly, but right now we don't use these events.
        // FIXME: When a script stops due to an error, we should update our state accordingly.
        // (Right now we only update the state when the script is explicitly started/stopped.)
        thread::spawn(move || {
            loop {
                let _ = rx_env.recv();
            }
        });

        Ok(())
    }
}
