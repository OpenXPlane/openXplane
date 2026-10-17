//! Whole nested root refresh14113a300. Leaf/context/assignment/destruction
//! providers remain services; traversal and temporary cleanup order are ported.
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct State {
    pub image: Vec<u8>,
    pub active_global: u64,
    pub resource_index: u32,
    pub links: BTreeMap<u64, u64>,
}
#[derive(Clone, Debug)]
pub struct Temporary {
    pub identity: u64,
    pub raw: Vec<u8>,
}
pub trait Services {
    fn release_item(&mut self, state: &mut State, item: u64);
    fn engine(&mut self, state: &mut State) -> u8;
    fn resource(&mut self, state: &mut State, index: u32) -> u64;
    fn available(&self, resource: u64) -> bool;
    fn assign_resource(&mut self, state: &mut State, resource: u64);
    fn settings(&mut self, state: &mut State, index: u32) -> u64;
    fn setting_mode(&self, settings: u64) -> i32;
    fn temporary(&mut self, state: &mut State, resource: u64) -> Temporary;
    fn assign_payload(&mut self, state: &mut State, temporary: &mut Temporary);
    fn free_temporary_vector(&mut self, state: &mut State, address: u64);
    fn destroy_optional(&mut self, state: &mut State, temporary: &mut Temporary);
    fn notify(&mut self, state: &mut State, wrapper: u64);
}
fn q(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}
fn putq(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
pub fn refresh(state: &mut State, services: &mut impl Services) -> Result<(), &'static str> {
    if state.image.len() != 0x158 {
        return Err("nested refresh image size mismatch");
    }
    state.image[0x130] = 0;
    let mut cursor = q(&state.image, 0x140);
    let end = q(&state.image, 0x148);
    if cursor != end {
        let distance = end.checked_sub(cursor).ok_or("valid item range required")?;
        if distance % 8 != 0 || distance / 8 > 100000 {
            return Err("bounded aligned item range required");
        }
        while cursor != end {
            let item = *state.links.get(&cursor).ok_or("missing item slot")?;
            if item != 0 {
                services.release_item(state, item);
            }
            cursor = cursor.checked_add(8).ok_or("item cursor overflow")?;
        }
        // Calls may change begin. Native captures end before traversal, then reloads begin.
        let begin = q(&state.image, 0x140);
        putq(&mut state.image, 0x148, begin);
    }
    if services.engine(state) != 0 && state.active_global == 0 {
        let resource = services.resource(state, state.resource_index);
        if services.available(resource) {
            let resource = services.resource(state, state.resource_index);
            services.assign_resource(state, resource);
            let settings = services.settings(state, state.resource_index);
            if services.setting_mode(settings) == 0 {
                let resource = services.resource(state, state.resource_index);
                let mut temp = services.temporary(state, resource);
                if temp.raw.len() != 0xe0 {
                    return Err("temporary payload size mismatch");
                }
                services.assign_payload(state, &mut temp);
                let begin = q(&temp.raw, 0xa8);
                if begin
                    != temp
                        .identity
                        .checked_add(0xc0)
                        .ok_or("temporary identity overflow")?
                {
                    services.free_temporary_vector(state, begin);
                }
                services.destroy_optional(state, &mut temp);
            }
        }
    }
    state.image[0x18] = 1;
    let head = q(&state.image, 8);
    let mut current = *state.links.get(&head).ok_or("missing listener head")?;
    let mut visits = 0;
    while current != q(&state.image, 8) {
        visits += 1;
        if visits > 100000 {
            return Err("bounded listener traversal required");
        }
        let old = current;
        services.notify(
            state,
            current.checked_add(16).ok_or("wrapper identity overflow")?,
        );
        current = *state
            .links
            .get(&old)
            .ok_or("missing retained listener next")?;
    }
    state.image[0x18] = 0;
    Ok(())
}
/// Bridge into the first child's nested refresh service. Runtime global/index/
/// links persist; the final root refresh remains independently supplied.
pub fn refresh_image(
    image: &mut [u8],
    active_global: &mut u64,
    resource_index: &mut u32,
    links: &mut BTreeMap<u64, u64>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    let mut state = State {
        image: image.to_vec(),
        active_global: *active_global,
        resource_index: *resource_index,
        links: std::mem::take(links),
    };
    let result = refresh(&mut state, services);
    image.copy_from_slice(&state.image);
    *active_global = state.active_global;
    *resource_index = state.resource_index;
    *links = state.links;
    result
}
