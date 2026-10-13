//! Native UI selection/focus transition. Tokens are opaque widget identities;
//! graph traversal and notification bodies are supplied by the live UI executor.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FocusState {
    pub selected: u64,
    pub focused: u64,
    pub mode: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notification {
    Leave(u64),
    Blur(u64),
    Enter(u64),
}

impl FocusState {
    /// Matches 0x1412f7fe0. `leaf` resolves the last widget in the +0x10 chain.
    /// Leave/enter correspond to helper calls with byte argument 1; blur is slot +0x40.
    /// Notifications may mutate state, and subsequent native reads observe those changes.
    /// The mode's higher-level meaning and notification helper internals remain unported.
    pub fn select(
        &mut self,
        target: u64,
        mode: u32,
        mut leaf: impl FnMut(u64) -> u64,
        mut notify: impl FnMut(&mut Self, Notification),
    ) {
        let previous = self.selected;
        if previous == target {
            return;
        }
        let skip_leave = self.mode == 0 && mode == 1;
        let skip_enter = self.mode == 1 && mode == 0;
        if previous != 0 && !skip_leave {
            notify(self, Notification::Leave(previous));
        }
        if self.focused != target {
            if self.focused != 0 {
                let last = leaf(self.focused);
                notify(self, Notification::Blur(last));
            }
            self.focused = target;
        }
        self.selected = target;
        self.mode = mode;
        if target != 0 && !skip_enter {
            notify(self, Notification::Enter(target));
        }
    }
}
