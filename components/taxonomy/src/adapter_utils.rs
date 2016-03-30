//! Utilities for writing adapters.

use api::{ Error, User };
use manager::*;
use services::{ Getter, Setter };
use util::{ Id, AdapterId };
use values::*;

use std::collections::HashMap;
use std::sync::{ Arc, Mutex };

use transformable_channels::mpsc::*;

/// A simple way of converting an Adapter to an Adapter + Sync.
///
/// Hardly optimal, but useful for testing and prototyping.
pub struct MakeSyncAdapter<T> where T: Adapter {
    lock: Mutex<Arc<T>>,
    id: Id<AdapterId>,
    name: String,
    vendor: String,
    version: [u32; 4],
}

impl<T> MakeSyncAdapter<T> where T: Adapter {
    pub fn new(adapter: T) -> Self {
        MakeSyncAdapter {
            id: adapter.id().clone(),
            name: adapter.name().to_owned(),
            vendor: adapter.vendor().to_owned(),
            version: adapter.version().to_owned(),
            lock: Mutex::new(Arc::new(adapter)),
        }
    }
}
impl<T> Adapter for MakeSyncAdapter<T> where T: Adapter {
    fn id(&self) -> Id<AdapterId> {
        self.id.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn vendor(&self) -> &str {
        &self.vendor
    }

    fn version(&self) -> &[u32;4] {
        &self.version
    }

    fn fetch_values(&self, set: Vec<Id<Getter>>, user: User) -> ResultMap<Id<Getter>, Option<Value>, Error> {
        self.lock.lock().unwrap().fetch_values(set, user)
    }

    fn send_values(&self, values: HashMap<Id<Setter>, Value>, user: User) -> ResultMap<Id<Setter>, (), Error> {
        self.lock.lock().unwrap().send_values(values, user)
    }

    fn register_watch(&self, set: Vec<(Id<Getter>, Option<Range>)>,
        cb: Box<ExtSender<WatchEvent>>) ->
            ResultMap<Id<Getter>, Box<AdapterWatchGuard>, Error> {
        self.lock.lock().unwrap().register_watch(set, cb)
    }
}
