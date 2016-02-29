use devices::{NodeId, ServiceKind, Service, Input, Output};
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
/// use fxbox_taxonomy::selector::*;
/// use fxbox_taxonomy::devices::*;
///
/// let selector = NodeSelector::new()
///   .with_tags(vec!["entrance".to_owned()])
///   .with_inputs(vec![InputSelector::new() /* can be more restrictive */]);
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
    pub inputs: Vec<InputSelector>,

    /// Restrict results to nodes that have all the outputs in `outputs`.
    #[serde(default)]
    pub outputs: Vec<OutputSelector>,

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
    pub fn with_inputs(mut self, mut inputs: Vec<InputSelector>) -> Self {
        NodeSelector {
            inputs: {self.inputs.append(&mut inputs); self.inputs},
            .. self
        }
    }

    /// Restrict results to nodes that have all the outputs in `outputs`.
    pub fn with_outputs(mut self, mut outputs: Vec<OutputSelector>) -> Self {
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



/// A selector for one or more input services.
///
///
/// # Example
///
/// ```
/// use fxbox_taxonomy::selector::*;
/// use fxbox_taxonomy::devices::*;
/// use fxbox_taxonomy::util::Id;
///
/// let selector = InputSelector::new()
///   .with_parent(Id<NodeId>::new("foxbox".to_owned()))
///   .with_kind(ServiceKind::CurrentTimeOfDay);
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InputSelector {
    /// If `Exactly(id)`, return only the service with the corresponding id.
    #[serde(default)]
    pub id: Exactly<Id<Input>>,

    /// If `Eactly(id)`, return only services that are children of
    /// node `id`.
    #[serde(default)]
    pub parent: Exactly<Id<NodeId>>,

    ///  Restrict results to services that have all the tags in `tags`.
    #[serde(default)]
    pub tags: Vec<String>,

    /// If `Exatly(k)`, restrict results to services that produce values
    /// of kind `k`.
    #[serde(default)]
    pub kind: Exactly<ServiceKind>,

    /// If `Some(r)`, restrict results to services that support polling
    /// with the acceptable period.
    #[serde(default)]
    pub poll: Option<Period>,

    /// If `Some(r)`, restrict results to services that support trigger
    /// with the acceptable period.
    #[serde(default)]
    pub trigger: Option<Period>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}
impl InputSelector {
    /// Create a new selector that accepts all input services.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict to a service with a specific id.
    pub fn with_id(self, id: Id<Input>) -> Self {
        InputSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a service with a specific parent.
    pub fn with_parent(self, id: Id<NodeId>) -> Self {
        InputSelector {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a service with a specific kind.
    pub fn with_kind(self, kind: ServiceKind) -> Self {
        InputSelector {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to services that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        InputSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict to services that support polling with the acceptable
    /// period
    pub fn with_poll(self, period: Period) -> Self {
        InputSelector {
            poll: Period::and_option(self.poll, Some(period)),
            .. self
        }
    }

    /// Restrict to services that support trigger with the acceptable
    /// period
    pub fn with_trigger(self, period: Period) -> Self {
        InputSelector {
            trigger: Period::and_option(self.trigger, Some(period)),
            .. self
        }
    }

    /// Restrict to services that are accepted by two selector.
    pub fn and(self, other: Self) -> Self {
        InputSelector {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: merge(self.tags, other.tags),
            kind: self.kind.and(other.kind),
            poll: Period::and_option(self.poll, other.poll),
            trigger: Period::and_option(self.trigger, other.trigger),
            private: (),
        }
    }

    /// Determine if a service is matched by this selector.
    pub fn matches(&self, service: &Service<Input>) -> bool {
        if !self.id.matches(&service.id) {
            return false;
        }
        if !self.parent.matches(&service.node) {
            return false;
        }
        if !self.kind.matches(&service.mechanism.kind) {
            return false;
        }
        if !Period::matches_option(&self.poll, &service.mechanism.poll) {
            return false;
        }
        if !Period::matches_option(&self.trigger, &service.mechanism.trigger) {
            return false;
        }
        if !has_selected_tags(&self.tags, &service.tags) {
            return false;
        }
        return true;
    }
}

/// A selector for one or more output services.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputSelector {
    /// If `Exactly(id)`, return only the service with the corresponding id.
    #[serde(default)]
    pub id: Exactly<Id<Output>>,

    /// If `Exactly(id)`, return only services that are immediate children
    /// of node `id`.
    #[serde(default)]
    pub parent: Exactly<Id<NodeId>>,

    ///  Restrict results to services that have all the tags in `tags`.
    #[serde(default)]
    pub tags: Vec<String>,

    /// If `Exactly(k)`, restrict results to services that accept values
    /// of kind `k`.
    #[serde(default)]
    pub kind: Exactly<ServiceKind>,

    /// If `Some(r)`, restrict results to services that support pushing
    /// with the acceptable period.
    #[serde(default)]
    pub push: Option<Period>,

    /// Make sure that we can't instantiate from another crate.
    #[serde(default, skip_serializing)]
    private: (),
}

impl OutputSelector {
    /// Create a new selector that accepts all input services.
    pub fn new() -> Self {
        OutputSelector::default()
    }

    /// Selector to a service with a specific id.
    pub fn with_id(self, id: Id<Output>) -> Self {
        OutputSelector {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Selector to services with a specific parent.
    pub fn with_parent(self, id: Id<NodeId>) -> Self {
        OutputSelector {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Selector to services with a specific kind.
    pub fn with_kind(self, kind: ServiceKind) -> Self {
        OutputSelector {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to services that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        OutputSelector {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict to services that support push with the acceptable
    /// period
    pub fn with_push(self, period: Period) -> Self {
        OutputSelector {
            push: Period::and_option(self.push, Some(period)),
            .. self
        }
    }

    /// Restrict results to services that are accepted by two selector.
    pub fn and(self, other: Self) -> Self {
        OutputSelector {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: merge(self.tags, other.tags),
            kind: self.kind.and(other.kind),
            push: Period::and_option(self.push, other.push),
            private: (),
        }
    }

    /// Determine if a service is matched by this selector.
    pub fn matches(&self, service: &Service<Output>) -> bool {
        if !self.id.matches(&service.id) {
            return false;
        }
        if !self.parent.matches(&service.node) {
            return false;
        }
        if !self.kind.matches(&service.mechanism.kind) {
            return false;
        }
        if !Period::matches_option(&self.push, &service.mechanism.push) {
            return false;
        }
        if !has_selected_tags(&self.tags, &service.tags) {
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
