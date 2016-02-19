use devices::{NodeId, ServiceId, ServiceKind};

use std::time::Duration;
use std::cmp;

/// A marker for a request that a expects a specific value.
#[derive(Clone, Debug)]
enum Exactly<Id> {
    /// No constraint.
    Empty,

    /// Expect a specific value.
    Exactly(Id),

    /// Two conflicting constraints (or more) have been put on the value.
    Conflict,
}

impl<T> Exactly<T> where T: PartialEq {
    /// Combine two constraints.
    fn and(self, other: Self) -> Self {
        use self::Exactly::*;
        match (self, other) {
            (Conflict, _) | (_, Conflict) => Conflict,
            (Empty, x@_) | (x@_, Empty) => x,
            (Exactly(x), Exactly(y)) =>
                if x == y {
                    Exactly(y)
                } else {
                    Conflict
                }
        }
    }
}

impl<T> Default for Exactly<T> {
    fn default() -> Self {
        Exactly::Empty
    }
}

fn merge<T>(mut a: Vec<T>, mut b: Vec<T>) -> Vec<T> where T: Ord {
    a.append(&mut b);
    a.sort();
    a.dedup();
    a
}

/// A request for one or more nodes.
///
///
/// # Example
///
/// ```
/// use fxbox_taxonomy::requests::*;
/// use fxbox_taxonomy::devices::*;
///
/// let request = NodeRequest::new()
///   .with_tags(vec!["entrance".to_owned()])
///   .with_inputs(vec![InputRequest::new() /* can be more restrictive */]);
/// ```
#[derive(Clone, Debug, Default)]
pub struct NodeRequest {
    /// If `Exactly(id)`, return only the node with the corresponding id.
    id: Exactly<NodeId>,

    ///  Restrict results to nodes that have all the tags in `tags`.
    tags: Vec<String>,

    /// Restrict results to nodes that have all the inputs in `inputs`.
    inputs: Vec<InputRequest>,

    /// Restrict results to nodes that have all the outputs in `outputs`.
    outputs: Vec<OutputRequest>,
}


impl NodeRequest {
    /// Create a new request that accepts all nodes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request for a node with a specific id.
    pub fn with_id(self, id: NodeId) -> Self {
        NodeRequest {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    ///  Restrict results to nodes that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        NodeRequest {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict results to nodes that have all the inputs in `inputs`.
    pub fn with_inputs(mut self, mut inputs: Vec<InputRequest>) -> Self {
        NodeRequest {
            inputs: {self.inputs.append(&mut inputs); self.inputs},
            .. self
        }
    }

    /// Restrict results to nodes that have all the outputs in `outputs`.
    pub fn with_outputs(mut self, mut outputs: Vec<OutputRequest>) -> Self {
        NodeRequest {
            outputs: {self.outputs.append(&mut outputs); self.outputs},
            .. self
        }
    }

    /// Restrict results to nodes that are accepted by two requests.
    pub fn and(mut self, mut other: NodeRequest) -> Self {
        NodeRequest {
            id: self.id.and(other.id),
            tags: merge(self.tags, other.tags),
            inputs: {self.inputs.append(&mut other.inputs); self.inputs},
            outputs: {self.outputs.append(&mut other.outputs); self.outputs},
        }
    }
}



/// A request for one or more input services.
///
///
/// # Example
///
/// ```
/// use fxbox_taxonomy::requests::*;
/// use fxbox_taxonomy::devices::*;
///
/// let request = InputRequest::new()
///   .with_parent("foxbox".to_owned())
///   .with_kind(ServiceKind::CurrentTimeOfDay);
/// ```
#[derive(Clone, Debug, Default)]
pub struct InputRequest {
    /// If `Exactly(id)`, return only the service with the corresponding id.
    id: Exactly<ServiceId>,

    /// If `Eactly(id)`, return only services that are children of
    /// node `id`.
    parent: Exactly<NodeId>,

    ///  Restrict results to services that have all the tags in `tags`.
    tags: Vec<String>,

    /// If `Exatly(k)`, restrict results to services that produce values
    /// of kind `k`.
    kind: Exactly<ServiceKind>,

    /// If `Some(r)`, restrict results to services that support polling
    /// with the acceptable period.
    poll: Option<Period>,

    /// If `Some(r)`, restrict results to services that support trigger
    /// with the acceptable period.
    trigger: Option<Period>,
}

impl InputRequest {
    /// Create a new request that accepts all input services.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict to a service with a specific id.
    pub fn with_id(self, id: ServiceId) -> Self {
        InputRequest {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a service with a specific parent.
    pub fn with_parent(self, id: NodeId) -> Self {
        InputRequest {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Restrict to a service with a specific kind.
    pub fn with_kind(self, kind: ServiceKind) -> Self {
        InputRequest {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to services that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        InputRequest {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict to services that support polling with the acceptable
    /// period
    pub fn with_poll(self, period: Period) -> Self {
        InputRequest {
            poll: Period::and_option(self.poll, Some(period)),
            .. self
        }
    }

    /// Restrict to services that support trigger with the acceptable
    /// period
    pub fn with_trigger(self, period: Period) -> Self {
        InputRequest {
            trigger: Period::and_option(self.trigger, Some(period)),
            .. self
        }
    }

    /// Restrict to services that are accepted by two requests.
    pub fn and(self, other: Self) -> Self {
        InputRequest {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: merge(self.tags, other.tags),
            kind: self.kind.and(other.kind),
            poll: Period::and_option(self.poll, other.poll),
            trigger: Period::and_option(self.trigger, other.trigger),
        }
    }
}

/// A request for one or more output services.
#[derive(Clone, Debug, Default)]
pub struct OutputRequest {
    /// If `Exactly(id)`, return only the service with the corresponding id.
    id: Exactly<ServiceId>,

    /// If `Exactly(id)`, return only services that are immediate children
    /// of node `id`.
    parent: Exactly<NodeId>,

    ///  Restrict results to services that have all the tags in `tags`.
    tags: Vec<String>,

    /// If `Exactly(k)`, restrict results to services that accept values
    /// of kind `k`.
    kind: Exactly<ServiceKind>,

    /// If `Some(r)`, restrict results to services that support pushing
    /// with the acceptable period.
    push: Option<Period>,
}

impl OutputRequest {
    /// Create a new request that accepts all input services.
    pub fn new() -> Self {
        OutputRequest::default()
    }

    /// Request to a service with a specific id.
    pub fn with_id(self, id: ServiceId) -> Self {
        OutputRequest {
            id: self.id.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Request to services with a specific parent.
    pub fn with_parent(self, id: NodeId) -> Self {
        OutputRequest {
            parent: self.parent.and(Exactly::Exactly(id)),
            .. self
        }
    }

    /// Request to services with a specific kind.
    pub fn with_kind(self, kind: ServiceKind) -> Self {
        OutputRequest {
            kind: self.kind.and(Exactly::Exactly(kind)),
            .. self
        }
    }

    ///  Restrict to services that have all the tags in `tags`.
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        OutputRequest {
            tags: merge(self.tags, tags),
            .. self
        }
    }

    /// Restrict to services that support push with the acceptable
    /// period
    pub fn with_push(self, period: Period) -> Self {
        OutputRequest {
            push: Period::and_option(self.push, Some(period)),
            .. self
        }
    }

    /// Restrict results to services that are accepted by two requests.
    pub fn and(self, other: Self) -> Self {
        OutputRequest {
            id: self.id.and(other.id),
            parent: self.parent.and(other.parent),
            tags: merge(self.tags, other.tags),
            kind: self.kind.and(other.kind),
            push: Period::and_option(self.push, other.push),
        }
    }
}

/// An acceptable interval of time.
#[derive(Clone, Debug, Default)]
pub struct Period {
    pub min: Option<Duration>,
    pub max: Option<Duration>,
}
impl Period {
    fn and(self, other: Self) -> Self {
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

    fn and_option(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        match (a, b) {
            (None, x@_) => x,
            (x@_, None) => x,
            (Some(a), Some(b)) => Some(a.and(b))
        }
    }
}
