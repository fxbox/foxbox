//! Selectors for services and channels.
//!
//! The high-level API of Project Link always offers access by selectors, rather than by individual
//! services/channels. This allows operations such as sending a temperature to all heaters in the
//! living room (that's a selector), rather than needing to access every single heater one by one.

use api::services::*;
use io::parse::*;
use misc::util::ptr_eq;

use std::fmt::Debug;
use std::hash::Hash;
use std::collections::HashSet;

use serde::de::Deserialize;

fn merge<T>(mut a: HashSet<T>, b: &[T]) -> HashSet<T> where T: Hash + Eq + Clone {
    a.extend(b.iter().cloned());
    a
}

/// A selector for one or more services.
///
///
/// # Example
///
/// ```
/// use foxbox_taxonomy::api::selector::*;
/// use foxbox_taxonomy::misc::util::*;
/// use foxbox_taxonomy::io::parse::*;
///
/// let selector = ServiceSelector::new()
///   .with_tags(&[Id::new("entrance")])
///   .with_features(&[SimpleFeatureSelector::new() /* can be more restrictive */]);
/// ```
///
/// # JSON
///
/// A selector is an object with the following fields:
///
/// - (optional) string `id`: accept only a service with a given id;
/// - (optional) array of string `tags`:  accept only services with all the tags in the array;
/// - (optional) array of objects `getters` (see `GetterSelector`): accept only services with
///    channels matching all the selectors in this array;
/// - (optional) array of objects `setters` (see `SetterSelector`): accept only services with
///    channels matching all the selectors in this array;
///
/// While each field is optional, at least one field must be provided.
///
/// ```
/// use foxbox_taxonomy::api::selector::*;
/// use foxbox_taxonomy::io::parse::*;
///
/// // A selector with all fields defined.
/// let json_selector = "{
///   \"id\": \"setter 1\",
///   \"tags\": [\"tag 1\", \"tag 2\"],
///   \"getters\": [{
///     \"kind\": \"Ready\"
///   }],
///   \"setters\": [{
///     \"tags\": [\"tag 3\"]
///   }]
/// }";
///
/// ServiceSelector::from_str(json_selector, &EmptyDeserializeSupportForTests).unwrap();
/// ```
#[derive(Clone, Debug, Deserialize, Default)]
pub struct ServiceSelector {
    /// If `Exactly(id)`, return only the service with the corresponding id.
    pub id: Exactly<Id<ServiceId>>,

    ///  Restrict results to services that have all the tags in `tags`.
    pub tags: HashSet<Id<TagId>>,

    /// Restrict results to services that have all the getters in `getters`.
    pub features: Vec<SimpleFeatureSelector>,

    /// Make sure that we can't instantiate from another crate.
    private: (),
}

impl PartialEq for ServiceSelector {
    fn eq(&self, other: &Self) -> bool {
        // We always expect two ServiceSelectors to be distinct
        ptr_eq(self, other)
    }
}

impl Parser<ServiceSelector> for ServiceSelector {
    fn description() -> String {
        "ServiceSelector".to_owned()
    }
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<Self, ParseError> {
        let id = try!(match path.push("id", |path| Exactly::take_opt(path, source, "id", support)) {
            None => Ok(Exactly::Always),
            Some(result) => result
        });
        let tags : HashSet<_> = match path.push("tags", |path| Id::take_vec_opt(path, source, "tags", support)) {
            None => HashSet::new(),
            Some(Ok(mut vec)) => vec.drain(..).collect(),
            Some(Err(err)) => return Err(err),
        };
        let features = match path.push("features", |path| SimpleFeatureSelector::take_vec_opt(path, source, "features", support)) {
            None => vec![],
            Some(Ok(vec)) => vec,
            Some(Err(err)) => return Err(err)
        };

        Ok(ServiceSelector {
            id: id,
            tags: tags,
            features: features,
            private: ()
        })
    }
}

impl ServiceSelector {
    /// Create a new selector that accepts all services.
    pub fn new() -> Self {
        Self::default()
    }

    /// Selector for a service with a specific id.
    pub fn with_id(self, id: Id<ServiceId>) -> Self {
        ServiceSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    ///  Restrict results to services that have all the tags in `tags`.
    pub fn with_tags(self, tags: &[Id<TagId>]) -> Self {
        ServiceSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict results to services that have all the getters in `getters`.
    pub fn with_features(mut self, features: &[SimpleFeatureSelector]) -> Self {
        ServiceSelector {
            features: {self.features.extend_from_slice(features); self.features},
            .. self
        }
    }

    /// Restrict results to services that are accepted by two selector.
    pub fn and(mut self, mut other: ServiceSelector) -> Self {
        ServiceSelector {
            id: self.id.and(other.id),
            tags: self.tags.union(&other.tags).cloned().collect(),
            features: {self.features.append(&mut other.features); self.features},
            private: (),
        }
    }
}



/// A selector for one or more features channels.
///
///
/// # Example
///
/// ```
/// use foxbox_taxonomy::api::selector::*;
/// use foxbox_taxonomy::misc::util::Id;
///
/// let selector = FeatureSelector::new()
///   .with_tags(&[Id::new("tag 1"), Id::new("tag 2")])
///   .with_implements(Id::new("light/is-on"));
/// ```
///
/// # JSON
///
/// A selector is an object with the following fields:
///
/// - (optional) string `id`: accept only a channel with a given id;
/// - (optional) string `service`: accept only channels of a service with a given id;
/// - (optional) array of string `tags`:  accept only channels with all the tags in the array;
/// - (optional) array of string `service_tags`:  accept only channels of a service with all the
///        tags in the array;
/// - (optional) string|object `kind` (see `ChannelKind`): accept only channels of a given kind.
///
/// While each field is optional, at least one field must be provided.
///
/// ```
/// use foxbox_taxonomy::api::selector::*;
/// use foxbox_taxonomy::io::parse::*;
///
/// let source = r#"{
///   "tags": ["tag 1", "tag 2"],
///   "implements": "light/is-on"
/// }"#;
///
/// FeatureSelector::from_str(source, &EmptyDeserializeSupportForTests).unwrap();
/// ```
#[derive(Clone, Debug, Deserialize, Default)]
pub struct BaseFeatureSelector<T> where T: Clone + Debug + Deserialize + Default {
    /// If `Exactly(id)`, return only the channel with the corresponding id.
    pub id: Exactly<Id<FeatureId>>,

    /// Restrict results to features that appear in `services`.
    pub services: Exactly<T>,

    ///  Restrict results to channels that have all the tags in `tags`.
    pub tags: HashSet<Id<TagId>>,

    /// If `Exatly(k)`, restrict results to channels that produce values
    /// of kind `k`.
    pub implements: Exactly<Id<ImplementId>>,

    private: (),
}

pub type SimpleFeatureSelector = BaseFeatureSelector<()>;
pub type FeatureSelector = BaseFeatureSelector<Vec<ServiceSelector>>;

impl Parser<FeatureSelector> for FeatureSelector {
    fn description() -> String {
        "FeatureSelector".to_owned()
    }
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<Self, ParseError> {
        let services = try!(match path.push("services", |path| ServiceSelector::take_vec_opt(path, source, "services", support)) {
            None => Ok(Exactly::Always),
            Some(result) => Ok(Exactly::Exactly(try!(result)))
        });
        let base = try!(SimpleFeatureSelector::parse(path, source, support));
        Ok(BaseFeatureSelector {
            services: services,
            id: base.id,
            tags: base.tags,
            implements: base.implements,
            private: ()
        })
    }
}

impl Parser<SimpleFeatureSelector> for SimpleFeatureSelector {
    fn description() -> String {
        "SimpleFeatureSelector".to_owned()
    }
    fn parse(path: Path, source: &JSON, support: &DeserializeSupport) -> Result<Self, ParseError> {
        let id = try!(match path.push("id", |path| Exactly::take_opt(path, source, "id", support)) {
            None => Ok(Exactly::Always),
            Some(result) => {
                result
            }
        });
        let tags : HashSet<_> = match path.push("tags", |path| Id::take_vec_opt(path, source, "tags", support)) {
            None => HashSet::new(),
            Some(Ok(mut vec)) => {
                vec.drain(..).collect()
            }
            Some(Err(err)) => return Err(err),
        };
        let implements = try!(match path.push("implements", |path| Exactly::take_opt(path, source, "implements", support)) {
            None => Ok(Exactly::Always),
            Some(result) => {
                result
            }
        });
        Ok(BaseFeatureSelector {
            id: id,
            services: Exactly::Always,
            tags: tags,
            implements: implements,
            private: ()
        })
    }
}

impl<T> BaseFeatureSelector<T> where T: Clone + Debug + Deserialize + Default {
    /// Create a new selector that accepts all getter channels.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict to a channel with a specific id.
    pub fn with_id(self, id: Id<FeatureId>) -> Self {
        BaseFeatureSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a channel with a specific kind.
    pub fn with_implements(self, id: Id<ImplementId>) -> Self {
        BaseFeatureSelector {
            implements: self.implements.and(Exactly::Exactly(id)),
            .. self
        }
    }

    ///  Restrict to channels that have all the tags in `tags`.
    pub fn with_tags(self, tags: &[Id<TagId>]) -> Self {
        BaseFeatureSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }
}

impl BaseFeatureSelector<Vec<ServiceSelector>> {
    /// Restrict to a channel with a specific parent.
    pub fn with_service(self, services: Vec<ServiceSelector>) -> Self {
        BaseFeatureSelector {
            services: self.services.and(Exactly::Exactly(services)),
            .. self
        }
    }
}

