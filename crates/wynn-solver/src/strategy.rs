use wynn_core::item::Slot;

/// Search strategy: determines which slots to flex and how to narrow the search.
#[derive(Debug, Clone)]
pub struct SearchStrategy {
    /// Slots that should not be changed.
    pub locked: Vec<Slot>,
    /// Slots the solver can swap items in, in priority order.
    pub flexible: Vec<Slot>,
}

impl SearchStrategy {
    /// Create a strategy that locks specific slots and flexes the rest.
    pub fn with_locked(locked: Vec<Slot>) -> Self {
        let flexible: Vec<Slot> = Slot::ALL
            .iter()
            .filter(|s| !locked.contains(s))
            .copied()
            .collect();
        Self { locked, flexible }
    }
}
