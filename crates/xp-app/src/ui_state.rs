//! Native window-presence/opacity query services used by menu command routing.
//! Runtime node creation, window effects and rendering are not yet connected.
use crate::menu_commands::{self, Command, Request};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct RegisteredWindow {
    /// Opaque window token; models the native pointer identity without storing host pointers.
    pub token: u64,
    /// Opacities at native node +0x58, root first, then descendants down to the window.
    pub opacity_root_to_leaf: Vec<f32>,
}

/// Recursive native helper 0x1412d9430, expressed root-to-leaf to preserve f32 order.
/// Empty paths have no corresponding native widget and are rejected.
pub fn cumulative_opacity(path: &[f32]) -> Result<f32, &'static str> {
    let (&root, rest) = path
        .split_first()
        .ok_or("empty native widget opacity path")?;
    Ok(rest.iter().fold(root, |value, &local| value * local))
}

#[derive(Clone, Debug, Default)]
pub struct WindowState {
    /// Native controller map at +0x148; token 0 represents a null mapped window.
    pub by_kind: BTreeMap<i32, u64>,
    /// Window-system list order is significant: the first matching non-null entry wins.
    pub registered: Vec<Option<RegisteredWindow>>,
    /// Raw byte at native engine-state +0x525; its lifecycle/meaning remains unresolved.
    pub state_byte_525: u8,
}

impl WindowState {
    /// Native query 0x141512eb0, including membership and cumulative opacity services.
    /// Finite input paths have original-code evidence; NaN payload parity is not claimed.
    pub fn query(&self, kind: i32) -> Result<bool, &'static str> {
        let Some(&token) = self.by_kind.get(&kind).filter(|&&token| token != 0) else {
            return Ok(false);
        };
        let Some(window) = self.registered.iter().flatten().find(|w| w.token == token) else {
            return Ok(false);
        };
        Ok(cumulative_opacity(&window.opacity_root_to_leaf)? > 0.0)
    }

    /// The recovered command routing now takes window state rather than fabricated query results.
    /// Window action bodies still need a native-equivalent executor before live UI integration.
    pub fn menu_requests(&self, command: Command) -> Result<Vec<Request>, &'static str> {
        let present = match command {
            Command::ToggleMainMenu => self.query(11)?,
            Command::ToggleSettings => false,
        };
        Ok(menu_commands::requests(
            command,
            present,
            self.state_byte_525 != 0,
        ))
    }
}
