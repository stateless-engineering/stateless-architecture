//! Lifecycle state machine for stateless systems.
//!
//! Transitions: Active → Freezing → Hibernated → Restoring → Active

/// System lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    /// Running and accepting service invocations.
    Active,
    /// Snapshotting the blob, preparing to evict.
    Freezing,
    /// Evicted — only the blob exists on disk.
    Hibernated,
    /// Materializing from blob back to active.
    Restoring,
}

/// Check if a lifecycle transition is valid.
pub fn check_transition(from: State, to: State) -> Result<(), String> {
    match (from, to) {
        (State::Active, State::Freezing) => Ok(()),
        (State::Freezing, State::Hibernated) => Ok(()),
        (State::Freezing, State::Active) => Ok(()), // cancel
        (State::Hibernated, State::Restoring) => Ok(()),
        (State::Restoring, State::Active) => Ok(()),
        (State::Active, State::Active) => Ok(()), // no-op
        (from, to) => Err(format!("invalid transition: {:?} → {:?}", from, to)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_to_freezing() {
        assert!(check_transition(State::Active, State::Freezing).is_ok());
    }

    #[test]
    fn freezing_to_hibernated() {
        assert!(check_transition(State::Freezing, State::Hibernated).is_ok());
    }

    #[test]
    fn hibernated_to_restoring() {
        assert!(check_transition(State::Hibernated, State::Restoring).is_ok());
    }

    #[test]
    fn restoring_to_active() {
        assert!(check_transition(State::Restoring, State::Active).is_ok());
    }

    #[test]
    fn invalid_skip() {
        assert!(check_transition(State::Active, State::Hibernated).is_err());
    }
}
