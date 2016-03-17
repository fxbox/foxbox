use services::{ AdapterId, Service, ServiceId, ChannelKind, Channel, Getter, Setter, TagId };
use util::{Exactly, Id};
use values;

use serde::ser::Serializer;
use serde::de::Deserializer;

use std::cmp;
use std::hash::Hash;
use std::collections::HashSet;

fn merge<T>(mut a: HashSet<T>, b: Vec<T>) -> HashSet<T> where T: Hash + Eq {
    for x in b {
        a.insert(x);
    }
    a
}

pub trait SelectedBy<T> {
    fn matches(&self, &T) -> bool;
}

/// A trait used to let ServiceSelector work on complex data structures
/// that are not necessarily exactly Selector.
pub trait ServiceLike {
    fn id(&self) -> &Id<ServiceId>;
    fn tags(&self) -> &HashSet<Id<TagId>>;
    fn adapter(&self) -> &Id<AdapterId>;
    fn has_getters<F>(&self, f: F) -> bool where F: Fn(&Channel<Getter>) -> bool;
    fn has_setters<F>(&self, f: F) -> bool where F: Fn(&Channel<Setter>) -> bool;
}

impl ServiceLike for Service {
    fn id(&self) -> &Id<ServiceId> {
        &self.id
    }
    fn tags(&self) -> &HashSet<Id<TagId>> {
        &self.tags
    }
    fn adapter(&self) -> &Id<AdapterId> {
        &self.adapter
    }
    fn has_getters<F>(&self, f: F) -> bool where F: Fn(&Channel<Getter>) -> bool {
        for chan in self.getters.values() {
            if f(chan) {
                return true;
            }
        }
        false
    }
    fn has_setters<F>(&self, f: F) -> bool where F: Fn(&Channel<Setter>) -> bool {
        for chan in self.setters.values() {
            if f(chan) {
                return true;
            }
        }
        false
    }

}

/// A selector for one or more services.
///
///
/// # Example
///
/// ```
/// use foxbox_taxonomy::selector::*;
/// use foxbox_taxonomy::services::*;
/// use foxbox_taxonomy::util::Id;
///
/// let selector = ServiceSelector::new()
///   .with_tags(vec![Id::<TagId>::new("entrance")])
///   .with_getters(vec![GetterSelector::new() /* can be more restrictive */]);
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ServiceSelector {
    /// If `Exactly(id)`, return only the service with the corresponding id.
    #[serde(default)]
    pub id: Exactly<Id<ServiceId>>,

    ///  Restrict results to services that have all the tags in `tags`.
    #[serde(default)]
    pub tags: HashSet<Id<TagId>>,

    /// Restrict results to services that have all the getters in `getters`.
    #[serde(default)]
    pub getters: Vec<GetterSelector>,

    /// Restrict results to services that have all the setters in `setters`.
    #[serde(default)]
    pub setters: Vec<SetterSelector>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
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
    pub fn with_tags(self, tags: Vec<Id<TagId>>) -> Self {
        ServiceSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict results to services that have all the getters in `getters`.
    pub fn with_getters(mut self, mut getters: Vec<GetterSelector>) -> Self {
        ServiceSelector {
            getters: {self.getters.append(&mut getters); self.getters},
            .. self
        }
    }

    /// Restrict results to services that have all the setters in `setters`.
    pub fn with_setters(mut self, mut setters: Vec<SetterSelector>) -> Self {
        ServiceSelector {
            setters: {self.setters.append(&mut setters); self.setters},
            .. self
        }
    }

    /// Restrict results to services that are accepted by two selector.
    pub fn and(mut self, mut other: ServiceSelector) -> Self {
        ServiceSelector {
            id: self.id.and(other.id),
            tags: self.tags.union(&other.tags).cloned().collect(),
            getters: {self.getters.append(&mut other.getters); self.getters},
            setters: {self.setters.append(&mut other.setters); self.setters},
            private: (),
        }
    }

    pub fn matches<T>(&self, service: &T) -> bool
        where T: ServiceLike
    {
        if !self.id.matches(service.id()) {
            return false;
        }
        if !has_selected_tags(&self.tags, service.tags()) {
            return false;
        }
        // If any of the getter selectors doesn't find a getter,
        // we don't match.
        let getters_fail = self.getters.iter().find(|selector| {
            !service.has_getters(|channel| {
                selector.matches(channel)
            })
        }).is_some();
        if getters_fail {
            return false;
        }
        // If any of the setter selectors doesn't find a setter,
        // we don't match.
        let setters_fail = self.setters.iter().find(|selector| {
            !service.has_setters(|channel| {
                selector.matches(channel)
            })
        }).is_some();
        if setters_fail {
            return false;
        }
        true
    }
}

impl SelectedBy<ServiceSelector> for Service {
    fn matches(&self, selector: &ServiceSelector) -> bool {
        selector.matches(self)
    }
}


/// A selector for one or more getter channels.
///
///
/// # Example
///
/// ```
/// use foxbox_taxonomy::selector::*;
/// use foxbox_taxonomy::services::*;
/// use foxbox_taxonomy::util::Id;
///
/// let selector = GetterSelector::new()
///   .with_parent(Id::new("foxbox"))
///   .with_kind(ChannelKind::CurrentTimeOfDay);
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GetterSelector {
    /// If `Exactly(id)`, return only the channel with the corresponding id.
    #[serde(default)]
    pub id: Exactly<Id<Getter>>,

    /// If `Eactly(id)`, return only channels that are children of
    /// service `id`.
    #[serde(default)]
    pub parent: Exactly<Id<ServiceId>>,

    ///  Restrict results to channels that have all the tags in `tags`.
    #[serde(default)]
    pub tags: HashSet<Id<TagId>>,

    /// If `Exatly(k)`, restrict results to channels that produce values
    /// of kind `k`.
    #[serde(default)]
    pub kind: Exactly<ChannelKind>,

    /// If `Some(r)`, restrict results to channels that support polling
    /// with the acceptable period.
    #[serde(default)]
    pub poll: Option<Period>,

    /// If `Some(r)`, restrict results to channels that support trigger
    /// with the acceptable period.
    #[serde(default)]
    pub trigger: Option<Period>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}
impl GetterSelector {
    /// Create a new selector that accepts all getter channels.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict to a channel with a specific id.
    pub fn with_id(self, id: Id<Getter>) -> Self {
        GetterSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a channel with a specific parent.
    pub fn with_parent(self, id: Id<ServiceId>) -> Self {
        GetterSelector {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a channel with a specific kind.
    pub fn with_kind(self, kind: ChannelKind) -> Self {
        GetterSelector {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to channels that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<Id<TagId>>) -> Self {
        GetterSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict to channels that support polling with the acceptable
    /// period
    pub fn with_poll(self, period: Period) -> Self {
        GetterSelector {
            poll: Period::and_option(self.poll, Some(period)),
            .. self
        }
    }

    /// Restrict to channels that support trigger with the acceptable
    /// period
    pub fn with_trigger(self, period: Period) -> Self {
        GetterSelector {
            trigger: Period::and_option(self.trigger, Some(period)),
            .. self
        }
    }

    /// Restrict to channels that are accepted by two selector.
    pub fn and(self, other: Self) -> Self {
        GetterSelector {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: self.tags.union(&other.tags).cloned().collect(),
            kind: self.kind.and(other.kind),
            poll: Period::and_option(self.poll, other.poll),
            trigger: Period::and_option(self.trigger, other.trigger),
            private: (),
        }
    }

    /// Determine if a channel is matched by this selector.
    pub fn matches(&self, channel: &Channel<Getter>) -> bool {
        if !self.id.matches(&channel.id) {
            return false;
        }
        if !self.parent.matches(&channel.service) {
            return false;
        }
        if !self.kind.matches(&channel.mechanism.kind) {
            return false;
        }
        if !Period::matches_option(&self.poll, &channel.mechanism.poll) {
            return false;
        }
        if !Period::matches_option(&self.trigger, &channel.mechanism.trigger) {
            return false;
        }
        if !has_selected_tags(&self.tags, &channel.tags) {
            return false;
        }
        true
    }
}

impl SelectedBy<GetterSelector> for Channel<Getter> {
    fn matches(&self, selector: &GetterSelector) -> bool {
        selector.matches(self)
    }
}

/// A selector for one or more setter channels.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SetterSelector {
    /// If `Exactly(id)`, return only the channel with the corresponding id.
    #[serde(default)]
    pub id: Exactly<Id<Setter>>,

    /// If `Exactly(id)`, return only channels that are immediate children
    /// of service `id`.
    #[serde(default)]
    pub parent: Exactly<Id<ServiceId>>,

    ///  Restrict results to channels that have all the tags in `tags`.
    #[serde(default)]
    pub tags: HashSet<Id<TagId>>,

    /// If `Exactly(k)`, restrict results to channels that accept values
    /// of kind `k`.
    #[serde(default)]
    pub kind: Exactly<ChannelKind>,

    /// If `Some(r)`, restrict results to channels that support pushing
    /// with the acceptable period.
    #[serde(default)]
    pub push: Option<Period>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}

impl SetterSelector {
    /// Create a new selector that accepts all getter channels.
    pub fn new() -> Self {
        SetterSelector::default()
    }

    /// Selector to a channel with a specific id.
    pub fn with_id(self, id: Id<Setter>) -> Self {
        SetterSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Selector to channels with a specific parent.
    pub fn with_parent(self, id: Id<ServiceId>) -> Self {
        SetterSelector {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Selector to channels with a specific kind.
    pub fn with_kind(self, kind: ChannelKind) -> Self {
        SetterSelector {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to channels that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<Id<TagId>>) -> Self {
        SetterSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict to channels that support push with the acceptable
    /// period
    pub fn with_push(self, period: Period) -> Self {
        SetterSelector {
            push: Period::and_option(self.push, Some(period)),
            .. self
        }
    }

    /// Restrict results to channels that are accepted by two selector.
    pub fn and(self, other: Self) -> Self {
        SetterSelector {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: self.tags.union(&other.tags).cloned().collect(),
            kind: self.kind.and(other.kind),
            push: Period::and_option(self.push, other.push),
            private: (),
        }
    }

    /// Determine if a channel is matched by this selector.
    pub fn matches(&self, channel: &Channel<Setter>) -> bool {
        if !self.id.matches(&channel.id) {
            return false;
        }
        if !self.parent.matches(&channel.service) {
            return false;
        }
        if !self.kind.matches(&channel.mechanism.kind) {
            return false;
        }
        if !Period::matches_option(&self.push, &channel.mechanism.push) {
            return false;
        }
        if !has_selected_tags(&self.tags, &channel.tags) {
            return false;
        }
        true
    }
}

impl SelectedBy<SetterSelector> for Channel<Setter> {
    fn matches(&self, selector: &SetterSelector) -> bool {
        selector.matches(self)
    }
}


/// An acceptable interval of time.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Period {
    #[serde(default)]
    pub min: Option<values::ValDuration>,
    #[serde(default)]
    pub max: Option<values::ValDuration>,
}
impl Period {
    pub fn and(self, other: Self) -> Self {
        let min = match (self.min, other.min) {
            (None, x) |
            (x, None) => x,
            (Some(min1), Some(min2)) => Some(cmp::max(min1, min2))
        };
        let max = match (self.max, other.max) {
            (None, x) |
            (x, None) => x,
            (Some(max1), Some(max2)) => Some(cmp::min(max1, max2))
        };
        Period {
            min: min,
            max: max
        }
    }

    pub fn and_option(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        match (a, b) {
            (None, x) |
            (x, None) => x,
            (Some(a), Some(b)) => Some(a.and(b))
        }
    }

    pub fn matches(&self, duration: &values::ValDuration) -> bool {
        if let Some(ref min) = self.min {
            if min > duration {
                return false;
            }
        }
        if let Some(ref max) = self.max {
            if max < duration {
                return false;
            }
        }
        true
    }

    pub fn matches_option(period: &Option<Self>, duration: &Option<values::ValDuration>) -> bool {
        match (period, duration) {
            (&Some(ref period), &Some(ref duration))
                if !period.matches(duration) => false,
            (&Some(_), &None) => false,
            _ => true,
        }
    }
}


fn has_selected_tags(actual: &HashSet<Id<TagId>>, requested: &HashSet<Id<TagId>>) -> bool {
    for tag in &*actual {
        if !requested.contains(tag) {
            return false;
        }
    }
    true
}
