//! The original keyboard binding-vector loop, 0x140c62868..0x140c628ea.
//! Entry gates, plugin/UI interception and command callback bodies are outside this port.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub key: u32,
    pub modifiers: u32,
    pub command: i32,
    /// Native int32 field: any nonzero value is active.
    pub active: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Begin(i32),
    End(i32),
}

/// Returns the calls made by the native loop with inert callback bodies, in order.
/// Effects of callbacks on the owner/vector during dispatch are not modeled. Native flag 8 is key-down;
/// flag 16 is key-up. Low three bits are modifiers. Other bits are ignored here.
/// Key-up releases active entries for that key even when modifiers have changed.
/// Key-down begins the first matching *inactive* entry and leaves the loop.
/// Thus a second key-down can begin a later duplicate while the first stays active.
/// Beginning a command clears the owner's release-suppression field.
pub fn route(
    entries: &mut [Entry],
    key: u32,
    flags: u32,
    suppress_release: &mut i32,
) -> Vec<Action> {
    let mut calls = Vec::new();
    for entry in entries {
        if entry.key != key {
            continue;
        }
        if entry.modifiers == flags & 7 && flags & 8 != 0 && entry.active == 0 {
            *suppress_release = 0;
            entry.active = 1;
            calls.push(Action::Begin(entry.command));
            break;
        }
        if entry.active != 0 && flags & 16 != 0 {
            entry.active = 0;
            if *suppress_release == 0 {
                calls.push(Action::End(entry.command));
            }
        }
    }
    calls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_down_and_modifier_changed_up_preserve_native_order() {
        let mut entries = [
            Entry {
                key: 70,
                modifiers: 4,
                command: 17,
                active: 0,
            },
            Entry {
                key: 70,
                modifiers: 4,
                command: 3083,
                active: 0,
            },
        ];
        let mut suppression = 1;
        assert_eq!(
            route(&mut entries, 70, 12, &mut suppression),
            [Action::Begin(17)]
        );
        assert_eq!(suppression, 0);
        assert_eq!(
            route(&mut entries, 70, 12, &mut suppression),
            [Action::Begin(3083)]
        );
        assert!(route(&mut entries, 70, 12, &mut suppression).is_empty());
        assert_eq!(
            route(&mut entries, 70, 16, &mut suppression),
            [Action::End(17), Action::End(3083)]
        );
        assert!(entries.iter().all(|e| e.active == 0));
    }

    #[test]
    fn suppressed_up_still_clears_native_active_field() {
        let mut entries = [Entry {
            key: 27,
            modifiers: 0,
            command: 3,
            active: -1,
        }];
        let mut suppression = -1;
        assert!(route(&mut entries, 27, 16, &mut suppression).is_empty());
        assert_eq!(entries[0].active, 0);
        assert_eq!(suppression, -1);
    }
}
