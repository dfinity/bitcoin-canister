use crate::with_state_mut;

/// Ensures that there is only one instance of the heartbeat state machine.
// Note: the struct has one private field to ensure that nobody can construct it
// directly outside of this module.
#[must_use]
pub struct FetchBlocksGuard(());

impl FetchBlocksGuard {
    pub fn new() -> Option<Self> {
        with_state_mut(|s| {
            if s.syncing_state.is_fetching_blocks {
                return None;
            }
            s.syncing_state.is_fetching_blocks = true;
            Some(FetchBlocksGuard(()))
        })
    }
}

impl Drop for FetchBlocksGuard {
    fn drop(&mut self) {
        with_state_mut(|s| {
            s.syncing_state.is_fetching_blocks = false;
        });
    }
}
