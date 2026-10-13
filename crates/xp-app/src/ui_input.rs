//! Native focused-element keyboard dispatch and acceptance mask.
//! Widget construction, callbacks and game-level Alt/Esc actions remain outside this module.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyInput {
    pub character: i32,
    pub virtual_key: u32,
    pub flags: u32,
}

/// Native acceptance filter 0x1412e5a90. Classification evidence uses the baseline's
/// initial mapped CRT classification profile. Unsupported classifier inputs are rejected rather
/// than inventing Windows invalid-parameter behavior.
pub fn accepts_key(mask: u32, input: KeyInput) -> Result<bool, &'static str> {
    let bit = match input.virtual_key {
        0x09 => 0x01,
        0x0d | 0xbb | 0xbc => 0x02,
        0x08 | 0x2e => 0x04,
        0x1b => 0x08,
        0x21..=0x28 => 0x10,
        _ => match input.character {
            0x2b => 0x80,
            0x2d => 0x2080,
            0x30..=0x39 => 0x40,
            0x41..=0x5a | 0x61..=0x7a => 0x20,
            0x20 => 0x100,
            -1..=0xff => {
                if (0x20..=0x7e).contains(&input.character) || input.character >= 0x80 {
                    0x200
                } else if matches!(input.virtual_key, 0x43 | 0x56 | 0x58) && input.flags & 4 != 0 {
                    0x400
                } else {
                    0x800
                }
            }
            _ => return Err("unsupported native CRT character classification input"),
        },
    };
    Ok(mask & bit != 0)
}

/// Dispatch matching 0x1412c6300 and 0x1412ee8a0. A missing focus receives no calls.
/// A child nonzero result becomes canonical 1; otherwise the filter runs, then the
/// element's own callback if accepted. Its raw u8 result is preserved.
/// The mask accessor runs after a declined child call, so child mask changes are observed.
/// Vtable/graph replacement during callbacks still requires the corresponding live graph executor.
pub fn dispatch_focused(
    focused: bool,
    mut get_mask: impl FnMut() -> u32,
    input: KeyInput,
    child: Option<&mut dyn FnMut(KeyInput) -> u8>,
    mut own: impl FnMut(KeyInput) -> u8,
) -> Result<u8, &'static str> {
    if !focused {
        return Ok(0);
    }
    if let Some(child) = child
        && child(input) != 0
    {
        return Ok(1);
    }
    if accepts_key(get_mask(), input)? {
        Ok(own(input))
    } else {
        Ok(0)
    }
}
