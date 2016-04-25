use taxonomy::util::Id as TaxId;
use taxonomy::services::Getter;
use taxonomy::values::*;
use taxonomy::adapter::{ AdapterWatchGuard, WatchEvent };

use transformable_channels::mpsc::ExtSender;

use std::collections::HashMap;
use std::sync::{ Arc, Mutex, Weak };

pub type SyncSender = Mutex<Box<ExtSender<WatchEvent>>>;
type WatchersMap = HashMap<usize, Arc<SyncSender>>;
type RangedWeakSender = (Option<Range>, Weak<SyncSender>);
pub type RangedSyncSender = (Option<Range>, Arc<SyncSender>);

pub struct Watchers {
    current_index: usize,
    map: Arc<Mutex<WatchersMap>>,
    getter_map: HashMap<TaxId<Getter>, Vec<RangedWeakSender>>,
}

impl Watchers {
    pub fn new() -> Self {
        Watchers {
            current_index: 0,
            map: Arc::new(Mutex::new(HashMap::new())),
            getter_map: HashMap::new(),
        }
    }

    pub fn push(&mut self, tax_id: TaxId<Getter>, range: Option<Range>, watcher: Arc<SyncSender>) -> WatcherGuard {
        let index = self.current_index;
        self.current_index += 1;
        {
            let mut map = self.map.lock().unwrap();
            map.insert(index, watcher.clone());
        }

        let entry = self.getter_map.entry(tax_id).or_insert(Vec::new());
        entry.push((range, Arc::downgrade(&watcher)));

        WatcherGuard {
            key: index,
            map: self.map.clone()
        }
    }

    fn get(&self, index: usize) -> Option<Arc<SyncSender>> {
        let map = self.map.lock().unwrap();
        map.get(&index).cloned()
    }

    pub fn get_from_tax_id(&self, tax_id: &TaxId<Getter>) -> Option<Vec<RangedSyncSender>> {
        self.getter_map.get(tax_id).and_then(|vec| {
            let vec: Vec<_> = vec.iter().filter_map(|&(ref range, ref weak_sender)| {
                let range = range.clone();
                weak_sender.upgrade().map(|sender| (range, sender))
            }).collect();
            if vec.len() == 0 { None } else { Some(vec) }
        })
    }
}

pub struct WatcherGuard {
    key: usize,
    map: Arc<Mutex<WatchersMap>>,
}

impl Drop for WatcherGuard {
    fn drop(&mut self) {
        let mut map = self.map.lock().unwrap();
        map.remove(&self.key);
    }
}

impl AdapterWatchGuard for WatcherGuard {}
