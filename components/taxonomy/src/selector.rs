use devices::{NodeId, ChannelKind, Channel, Get, Set};
use util::{Exactly, Id};
use values;

use serde::ser::Serializer;
use serde::de::Deserializer;

use std::cmp;

fn merge<T>(mut a: Vec<T>, mut b: Vec<T>) -> Vec<T> where T: Ord {
    a.append(&mut b);
    a.sort();
    a.dedup();
    a
}

/// A selector for one or more nodes.
///
///
/// # Example
///
/// ```
/// use foxbox_taxonomy::selector::*;
/// use foxbox_taxonomy::devices::*;
///
/// let selector = NodeSelector::new()
///   .with_tags(vec!["entrance".to_owned()])
///   .with_inputs(vec![GetSelector::new() /* can be more restrictive */]);
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeSelector {
    /// If `Exactly(id)`, return only the node with the corresponding id.
    #[serde(default)]
    pub id: Exactly<Id<NodeId>>,

    ///  Restrict results to nodes that have all the tags in `tags`.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Restrict results to nodes that have all the inputs in `inputs`.
    #[serde(default)]
    pub inputs: Vec<GetSelector>,

    /// Restrict results to nodes that have all the outputs in `outputs`.
    #[serde(default)]
    pub outputs: Vec<SetSelector>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}


impl NodeSelector {
    /// Create a new selector that accepts all nodes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Selector for a node with a specific id.
    pub fn with_id(self, id: Id<NodeId>) -> Self {
        NodeSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    ///  Restrict results to nodes that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        NodeSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict results to nodes that have all the inputs in `inputs`.
    pub fn with_inputs(mut self, mut inputs: Vec<GetSelector>) -> Self {
        NodeSelector {
            inputs: {self.inputs.append(&mut inputs); self.inputs},
            .. self
        }
    }

    /// Restrict results to nodes that have all the outputs in `outputs`.
    pub fn with_outputs(mut self, mut outputs: Vec<SetSelector>) -> Self {
        NodeSelector {
            outputs: {self.outputs.append(&mut outputs); self.outputs},
            .. self
        }
    }

    /// Restrict results to nodes that are accepted by two selector.
    pub fn and(mut self, mut other: NodeSelector) -> Self {
        NodeSelector {
            id: self.id.and(other.id),
            tags: merge(self.tags, other.tags),
            inputs: {self.inputs.append(&mut other.inputs); self.inputs},
            outputs: {self.outputs.append(&mut other.outputs); self.outputs},
            private: (),
        }
    }
}



/// A selector for one or more input channels.
///
///
/// # Example
///
/// ```
/// use foxbox_taxonomy::selector::*;
/// use foxbox_taxonomy::devices::*;
/// use foxbox_taxonomy::util::Id;
///
/// let selector = GetSelector::new()
///   .with_parent(Id::new("foxbox".to_owned()))
///   .with_kind(ChannelKind::CurrentTimeOfDay);
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GetSelector {
    /// If `Exactly(id)`, return only the channel with the corresponding id.
    #[serde(default)]
    pub id: Exactly<Id<Get>>,

    /// If `Eactly(id)`, return only channels that are children of
    /// node `id`.
    #[serde(default)]
    pub parent: Exactly<Id<NodeId>>,

    ///  Restrict results to channels that have all the tags in `tags`.
    #[serde(default)]
    pub tags: Vec<String>,

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
impl GetSelector {
    /// Create a new selector that accepts all input channels.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict to a channel with a specific id.
    pub fn with_id(self, id: Id<Get>) -> Self {
        GetSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a channel with a specific parent.
    pub fn with_parent(self, id: Id<NodeId>) -> Self {
        GetSelector {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a channel with a specific kind.
    pub fn with_kind(self, kind: ChannelKind) -> Self {
        GetSelector {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to channels that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        GetSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict to channels that support polling with the acceptable
    /// period
    pub fn with_poll(self, period: Period) -> Self {
        GetSelector {
            poll: Period::and_option(self.poll, Some(period)),
            .. self
        }
    }

    /// Restrict to channels that support trigger with the acceptable
    /// period
    pub fn with_trigger(self, period: Period) -> Self {
        GetSelector {
            trigger: Period::and_option(self.trigger, Some(period)),
            .. self
        }
    }

    /// Restrict to channels that are accepted by two selector.
    pub fn and(self, other: Self) -> Self {
        GetSelector {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: merge(self.tags, other.tags),
            kind: self.kind.and(other.kind),
            poll: Period::and_option(self.poll, other.poll),
            trigger: Period::and_option(self.trigger, other.trigger),
            private: (),
        }
    }

    /// Determine if a channel is matched by this selector.
    pub fn matches(&self, channel: &Channel<Get>) -> bool {
        if !self.id.matches(&channel.id) {
            return false;
        }
        if !self.parent.matches(&channel.node) {
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
        return true;
    }
}

/// A selector for one or more output channels.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SetSelector {
    /// If `Exactly(id)`, return only the channel with the corresponding id.
    #[serde(default)]
    pub id: Exactly<Id<Set>>,

    /// If `Exactly(id)`, return only channels that are immediate children
    /// of node `id`.
    #[serde(default)]
    pub parent: Exactly<Id<NodeId>>,

    ///  Restrict results to channels that have all the tags in `tags`.
    #[serde(default)]
    pub tags: Vec<String>,

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

impl SetSelector {
    /// Create a new selector that accepts all input channels.
    pub fn new() -> Self {
        SetSelector::default()
    }

    /// Selector to a channel with a specific id.
    pub fn with_id(self, id: Id<Set>) -> Self {
        SetSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Selector to channels with a specific parent.
    pub fn with_parent(self, id: Id<NodeId>) -> Self {
        SetSelector {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Selector to channels with a specific kind.
    pub fn with_kind(self, kind: ChannelKind) -> Self {
        SetSelector {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to channels that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        SetSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict to channels that support push with the acceptable
    /// period
    pub fn with_push(self, period: Period) -> Self {
        SetSelector {
            push: Period::and_option(self.push, Some(period)),
            .. self
        }
    }

    /// Restrict results to channels that are accepted by two selector.
    pub fn and(self, other: Self) -> Self {
        SetSelector {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: merge(self.tags, other.tags),
            kind: self.kind.and(other.kind),
            push: Period::and_option(self.push, other.push),
            private: (),
        }
    }

    /// Determine if a channel is matched by this selector.
    pub fn matches(&self, channel: &Channel<Set>) -> bool {
        if !self.id.matches(&channel.id) {
            return false;
        }
        if !self.parent.matches(&channel.node) {
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
        return true;
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
            (None, x@_) => x,
            (x@_, None) => x,
            (Some(min1), Some(min2)) => Some(cmp::max(min1, min2))
        };
        let max = match (self.max, other.max) {
            (None, x@_) => x,
            (x@_, None) => x,
            (Some(max1), Some(max2)) => Some(cmp::min(max1, max2))
        };
        Period {
            min: min,
            max: max
        }
    }

    pub fn and_option(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        match (a, b) {
            (None, x@_) => x,
            (x@_, None) => x,
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
        return true;
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

fn has_selected_tags(actual: &Vec<String>, requested: &Vec<String>) -> bool {
    for tag in &*actual {
        if requested.iter().find(|x| *x == tag).is_none() {
            return false;
        }
    }
    return true;
}
