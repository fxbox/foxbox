//! Utilities for writing adapters.

use adapters::adapter::{ AdapterWatchGuard, WatchEvent };
use adapters::manager::*;
use api::error::*;
use api::native::User;
use api::services::*;
use io::types::Value;

use std::sync::{ Arc, Mutex };

use transformable_channels::mpsc::*;

// A helper macro to create a Id<ServiceId> without boilerplate.
#[macro_export]
macro_rules! service_id {
    ($val:expr) => (Id::<ServiceId>::new($val))
}

// A helper macro to create a Id<AdapterId> without boilerplate.
#[macro_export]
macro_rules! adapter_id {
    ($val:expr) => (Id::<AdapterId>::new($val))
}

// A helper macro to create a Id<TagId> without boilerplate.
#[macro_export]
macro_rules! tag_id {
    ($val:expr) => (Id::<TagId>::new($val))
}

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

    fn fetch_values(&self, batch: PerFeature<Option<Value>>, user: User)
        -> PerFeature<Result<Option<Value>, Error>>
    {
        self.lock.lock().unwrap().fetch_values(batch, user)
    }

    fn send_values(&self, batch: PerFeature<Option<Value>>, user: User)
        -> PerFeature<Result<Option<Value>, Error>>
    {
        self.lock.lock().unwrap().send_values(batch, user)
    }

    fn delete_values(&self, batch: PerFeature<Option<Value>>, user: User)
        -> PerFeature<Result<Option<Value>, Error>>
    {
        self.lock.lock().unwrap().delete_values(batch, user)
    }

    fn register_watch(&self, batch: PerFeature<(Option<Value>, Box<ExtSender<WatchEvent>>)>) ->
        PerFeatureResult<Box<AdapterWatchGuard>>
    {
        self.lock.lock().unwrap().register_watch(batch)
    }
}
