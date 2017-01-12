// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

///
/// A file storage adapter. It watches files in a set of directories.
/// Each file or directory is exposed as two getters:
/// - id `node-info@$sha1`, which returns the file metadata.
/// - id `node@$sha1`, which returns the file content.
/// For a directory, the content is the metadata of leaf files.
///

extern crate notify;
extern crate serde_json;
extern crate walkdir;

use foxbox_core::traits::Controller;
use foxbox_taxonomy::adapter::*;
use foxbox_taxonomy::manager::AdapterManager;
use foxbox_taxonomy::api::{Error, InternalError, User};
use foxbox_taxonomy::channel::*;
use foxbox_taxonomy::services::{AdapterId, Id, Service, ServiceId};
use foxbox_taxonomy::util::Maybe;
use foxbox_taxonomy::values::{Binary, Json, format, Value};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;
use std::thread;
use self::notify::{watcher, DebouncedEvent, Watcher, RecursiveMode};
use self::walkdir::WalkDir;

mod metadata;
use self::metadata::*;

static ADAPTER_ID: &'static str = "filestorage_adapter@link.mozilla.org";
static ADAPTER_NAME: &'static str = "File Storage Adapter";
static ADAPTER_VENDOR: &'static str = "team@link.mozilla.org";
static ADAPTER_VERSION: [u32; 4] = [0, 0, 0, 0];

// The internal messages for this adapter.
#[derive(Debug)]
enum FileStorageMessage {
    Add(FileMetadata),
    Remove(PathBuf),
    Update(PathBuf),
}

pub struct FileStorageAdapter {
    tx: Arc<Mutex<Sender<FileStorageMessage>>>,
    dirs: Vec<String>, // The directories we are watching.
    nodes: Arc<Mutex<HashMap<String, FileMetadata>>>,
}

impl Adapter for FileStorageAdapter {
    fn id(&self) -> Id<AdapterId> {
        adapter_id!(ADAPTER_ID)
    }

    fn name(&self) -> &str {
        ADAPTER_NAME
    }

    fn vendor(&self) -> &str {
        ADAPTER_VENDOR
    }

    fn version(&self) -> &[u32; 4] {
        &ADAPTER_VERSION
    }

    fn fetch_values(&self,
                    mut set: Vec<Id<Channel>>,
                    _: User)
                    -> ResultMap<Id<Channel>, Option<Value>, Error> {
        set.drain(..)
            .map(|id| {
                // Get the hash from the channel id.
                let channel_id = id.as_atom().as_ref();
                let (cid, is_meta) = if channel_id.starts_with("file:content:") {
                    (&channel_id[13..], false)
                } else if channel_id.starts_with("file:metadata:") {
                    (&channel_id[14..], true)
                } else {
                    // Unknown channel!
                    error!("Unknown channel id: {}", channel_id);
                    return (id.clone(),
                            Err(Error::Internal(InternalError::NoSuchChannel(id.clone()))));
                };

                let nodes = self.nodes.lock().unwrap();
                if let Some(meta) = nodes.get(cid) {
                    // Ok, let's send the payload.
                    if is_meta {
                        info!("Serving metadata for {}", meta.path.display());
                        return (id.clone(),
                                Ok(Some(Value::new(Json(serde_json::to_value(&meta))))));
                    } else {
                        info!("Serving content for {}", meta.path.display());
                        if meta.kind == FileKind::File {
                            return (id.clone(), self.file_response(meta));
                        } else {
                            return (id.clone(), self.directory_response(meta, &nodes));
                        }
                    }
                } else {
                    // No such file hash.
                    // TODO: return a 404 ?
                    error!("Unknown file hash: {}", cid);
                }

                (id.clone(), Err(Error::Internal(InternalError::NoSuchChannel(id.clone()))))
            })
            .collect()
    }

    fn send_values(&self,
                   mut values: HashMap<Id<Channel>, Value>,
                   _: User)
                   -> ResultMap<Id<Channel>, (), Error> {

        values.drain()
            .map(|(id, value)| (id.clone(), Err(Error::Internal(InternalError::NoSuchChannel(id)))))
            .collect()
    }
}

impl FileStorageAdapter {
    pub fn init<C>(controller: C, adapt: &Arc<AdapterManager>) -> Result<(), Error>
        where C: Controller
    {
        info!("Starting file storage adapter");

        let (tx, rx) = channel::<FileStorageMessage>();

        // TODO: support storing/retrieving multiple paths in the config file.
        let config_store = controller.get_config();
        let dirs = match config_store.get("filestorage", "path") {
            Some(path) => vec![path],
            None => {
                match env::var("FOXBOX_STORAGE") {
                    Ok(path) => vec![path],
                    Err(_) => vec![],
                }
            }
        };

        let nodes = Arc::new(Mutex::new(HashMap::new()));
        let fs = Arc::new(FileStorageAdapter {
            dirs: dirs,
            tx: Arc::new(Mutex::new(tx)),
            nodes: nodes.clone(),
        });

        try!(adapt.add_adapter(fs.clone()));
        let service_id = service_id!("filestorage@link.mozilla.org");
        let adapter_id = adapter_id!(ADAPTER_ID);
        try!(adapt.add_service(Service::empty(&service_id, &adapter_id)));

        // TODO: Add upload setter

        let adapt = adapt.clone();

        // Spawn a thread that will listen to messages from the file watchers.
        thread::Builder::new()
            .name("FileStorageAdapter_main".to_owned())
            .spawn(move || {
                loop {
                    if let Ok(msg) = rx.recv() {
                        info!("FileStorageAdapter_main msg: {:?}", msg);
                        match msg {
                            FileStorageMessage::Add(meta) => {
                                let hash = meta.hash.clone();
                                 let is_file = meta.kind == FileKind::File;
                                // Insert the metadata in our node tracker.
                                nodes.lock().unwrap().insert(hash.clone(), meta);
                                // Create the getters for this node.
                                adapt.add_channel(Channel {
                                    feature: Id::new("file/file-content"),
                                    supports_fetch: Some(Signature::returns(Maybe::Required(if is_file { format::BINARY.clone() }
                                    else { format::JSON.clone() }))),
                                    id: Id::new(&format!("file:content:{}", hash.clone())),
                                    service: service_id.clone(),
                                    adapter: adapter_id.clone(),
                                    ..Channel::default()
                                }).unwrap();
                                adapt.add_channel(Channel {
                                    feature: Id::new("file/file-metadata"),
                                    supports_fetch: Some(Signature::returns(Maybe::Required(format::JSON.clone()))),
                                    id: Id::new(&format!("file:metadata:{}", hash.clone())),
                                    service: service_id.clone(),
                                    adapter: adapter_id.clone(),
                                    ..Channel::default()
                                }).unwrap();
                            }
                            FileStorageMessage::Remove(path) => {
                                let hash = get_path_hash(&path);
                                let mut nodes = nodes.lock().unwrap();
                                if nodes.contains_key(&hash) {
                                    // Remove both content and metadata channels.
                                    nodes.remove(&hash);
                                    adapt.remove_channel(&Id::new(&format!("file:content:{}", hash.clone()))).unwrap();
                                    adapt.remove_channel(&Id::new(&format!("file:metadata:{}", hash.clone()))).unwrap();
                                } else {
                                    error!("Trying to remove untracked file: {}", path.display());
                                }
                            }
                            FileStorageMessage::Update(path) => {
                                let hash = get_path_hash(&path);
                                let mut nodes = nodes.lock().unwrap();
                                if nodes.contains_key(&hash) {
                                    // Remove both content and metadata channels.
                                    nodes.remove(&hash);
                                    if let Ok(meta) = get_file_metadata(&path) {
                                        nodes.insert(hash.clone(), meta);
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .unwrap();

        fs.walk_dirs();
        fs.setup_notifier();

        Ok(())
    }

    fn setup_notifier(&self) {
        let dirs = self.dirs.clone();
        let main_tx = self.tx.clone();
        thread::Builder::new()
            .name("FileStorageAdapter_notifier".to_owned())
            .spawn(move || {
                let (tx, rx) = channel();
                let mut watcher = watcher(tx, Duration::from_secs(5)).unwrap();

                for dir in dirs {
                    watcher.watch(dir, RecursiveMode::Recursive).unwrap();
                }
                loop {
                    if let Ok(event) = rx.recv() {
                        match event {
                            DebouncedEvent::Create(path) => {
                                if let Ok(meta) = get_file_metadata(&path) {
                                    main_tx.lock()
                                        .unwrap()
                                        .send(FileStorageMessage::Add(meta))
                                        .unwrap();
                                }
                            }
                            DebouncedEvent::Remove(path) => {
                                main_tx.lock()
                                    .unwrap()
                                    .send(FileStorageMessage::Remove(path))
                                    .unwrap();
                            }
                            DebouncedEvent::Rename(old_path, new_path) => {
                                main_tx.lock()
                                    .unwrap()
                                    .send(FileStorageMessage::Remove(old_path))
                                    .unwrap();
                                if let Ok(meta) = get_file_metadata(&new_path) {
                                    main_tx.lock()
                                        .unwrap()
                                        .send(FileStorageMessage::Add(meta))
                                        .unwrap();
                                }
                            }
                            DebouncedEvent::NoticeWrite(path) |
                            DebouncedEvent::Write(path) => {
                                main_tx.lock()
                                    .unwrap()
                                    .send(FileStorageMessage::Update(path))
                                    .unwrap();
                            }
                            _ => info!("Unprocessed: {:?}", event),
                        }
                    }
                }
            })
            .unwrap();
    }

    fn walk_dirs(&self) {
        for dir in &self.dirs {
            let dir = dir.clone();
            let tx = self.tx.clone();
            thread::Builder::new()
                .name("FileStorageAdapter_walkdir".to_owned())
                .spawn(move || {
                    for entry in WalkDir::new(dir) {
                        if let Ok(entry) = entry {
                            if let Ok(meta) = get_file_metadata(entry.path()) {
                                tx.lock().unwrap().send(FileStorageMessage::Add(meta)).unwrap();
                            }
                        }
                    }
                })
                .unwrap();
        }
    }

    fn file_response(&self, meta: &FileMetadata) -> Result<Option<Value>, Error> {
        match get_file_content(&meta.path) {
            Ok(rsp) => {
                let data = unsafe { rsp.as_slice() };
                Ok(Some(Value::new(Binary {
                    data: Vec::from(data),
                    mimetype: Id::new(&meta.mime),
                })))
            }
            Err(err) => Err(Error::Internal(InternalError::GenericError(format!("{}", err)))),
        }
    }

    fn directory_response(&self,
                          meta: &FileMetadata,
                          nodes: &HashMap<String, FileMetadata>)
                          -> Result<Option<Value>, Error> {
        let mut entries = Vec::new();
        for entry in WalkDir::new(&meta.path).min_depth(1).max_depth(1) {
            if let Ok(entry) = entry {
                if let Some(fmeta) = nodes.get(&get_path_hash(entry.path())) {
                    entries.push(fmeta.clone());
                }
            }
        }
        Ok(Some(Value::new(Json(serde_json::to_value(&entries)))))
    }
}