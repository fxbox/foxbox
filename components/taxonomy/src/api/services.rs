//! This module defines the metadata on devices and services.
//!
//! Note that all the data structures in this module represent
//! snapshots of subsets of the devices available. None of these data
//! structures are live, so there is always the possibility that
//! devices may have been added or removed from the foxbox by the time
//! these data structures are read.

use io::parse::*;
use io::serialize::*;
pub use misc::util::{ Exactly, Expects, Id };

use serde::ser::{ Serializer };
use serde::de::{ Deserializer, Error };

use std::hash::{ Hasher };
use std::collections::{ HashSet, HashMap };


/// Metadata on a service. A service is a device or collection of devices
/// that may offer services. The foxbox itself is a service offering
/// services such as a clock, communicating with the user through her
/// smart devices, etc.
///
/// # JSON
///
/// A service is represented by an object with the following fields:
///
/// - id: string - an id unique to this service;
/// - adapter: string;
/// - tags: array of strings;
/// - properties: object;
/// - getters: object (keys are string identifiers, for more details on values see Channel<Getter>);
/// - setters: object (keys are string identifiers, for more details on values see Channel<Setter>);
///
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceDescription {
    /// Tags describing the service.
    ///
    /// These tags can be set by the user, adapters or
    /// applications. They are used by applications to find services and
    /// services.
    ///
    /// For instance, a user may set tag "entrance" to all services
    /// placed in the entrance of his house, or a tag "blue" to a service
    /// controlling blue lights. An adapter may set tags "plugged" or
    /// "battery" to devices that respectively depend on a plugged
    /// power source or on a battery.
    pub tags: HashSet<Id<TagId>>,

    /// An id unique to this service.
    pub id: Id<ServiceId>,

    /// Service properties that are set at creation time.
    /// For instance, these can be device manufacturer, model, etc.
    pub properties: HashMap<String, String>,

    /// Features exposed by this service
    pub features: HashMap<Id<FeatureId>, FeatureDescription>,

    /// Identifier of the adapter for this service.
    pub adapter: Id<AdapterId>,
}

impl ServiceDescription {
    /// Create an empty service.
    pub fn empty(id: Id<ServiceId>, adapter: Id<AdapterId>) -> Self {
        ServiceDescription {
            tags: HashSet::new(),
            features: HashMap::new(),
            properties: HashMap::new(),
            id: id,
            adapter: adapter,
        }
    }
}

impl ToJSON for ServiceDescription {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        vec![
            ("id", self.id.to_json(parts)),
            ("adapter", self.adapter.to_json(parts)),
            ("tags", self.tags.to_json(parts)),
            ("properties", self.properties.to_json(parts)),
            ("features", self.features.to_json(parts)),
        ].to_json(parts)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeatureDescription {
    pub id: Id<FeatureId>,
    pub service: Id<ServiceId>,
    pub adapter: Id<AdapterId>,

    pub implements: Vec<Id<ImplementId>>,
    pub send: String,
    pub watch: String,
    pub fetch: String,
    pub delete: String,

    pub tags: HashSet<Id<TagId>>,
}

impl ToJSON for FeatureDescription {
    fn to_json(&self, parts: &SerializeSupport) -> JSON {
        vec![
            ("id", self.id.to_json(parts)),
            ("adapter", self.adapter.to_json(parts)),
            ("service", self.service.to_json(parts)),
            ("implements", self.implements.to_json(parts)),
            ("send", self.send.to_json(parts)),
            ("watch", self.watch.to_json(parts)),
            ("fetch", self.fetch.to_json(parts)),
            ("delete", self.delete.to_json(parts)),
            ("tags", self.tags.to_json(parts)),
        ].to_json(parts)
    }
}

/// A marker for Id.
/// Only useful for writing `Id<ServiceId>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct ServiceId;


/// A marker for Id.
/// Only useful for writing `Id<AdapterId>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct AdapterId;

// A marker for Id.
/// Only useful for writing `Id<TagId>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct TagId;

#[derive(Clone, Debug, Default)]
pub struct FeatureId;

#[derive(Clone, Debug, Default)]
pub struct ImplementId;
