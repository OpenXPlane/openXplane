//! The menu bar of the simulator window, laid out like X-Plane's: a bar along the top edge that shows when
//! the pointer reaches it, with drop-down menus whose items carry their key shortcut. The menu titles
//! follow X-Plane 12 as remembered; this is not copied from its resources. An item with a key name runs
//! that key's action (the same path as pressing it); an item without one is shown dimmed until its
//! feature exists.
use crate::hud::{Color, Hud};

pub struct Item {
    pub label: &'static str,
    pub shortcut: &'static str,
    /// Key name of the keymap/own keys that performs the item, `"Quit"` for leaving.
    pub key: Option<&'static str>,
}

const fn item(label: &'static str, shortcut: &'static str, key: Option<&'static str>) -> Item {
    Item {
        label,
        shortcut,
        key,
    }
}

pub const MENUS: &[(&str, &[Item])] = &[
    (
        "Settings",
        &[
            item("Keyboard help", "H", Some("H")),
            item("Graphics", "", None),
            item("Sound", "", None),
            item("Joystick", "", None),
            item("Network", "", None),
        ],
    ),
    (
        "Aircraft",
        &[
            item("Aircraft details", "", None),
            item("Weight and balance", "", None),
            item("Flight model", "", None),
        ],
    ),
    (
        "Location",
        &[
            item("Local map", "", None),
            item("Select global airport", "", None),
        ],
    ),
    (
        "Flight Configuration",
        &[
            item("Reset flight", "Del", Some("Delete")),
            item("Pause", "P", Some("P")),
            item("Default view", "W", Some("W")),
            item("Arrow keys: stick or view", "Tab", Some("Tab")),
        ],
    ),
    ("Plugins", &[item("Plugin admin", "", None)]),
    ("Developer", &[item("Datarefs", "", None)]),
    ("Exit", &[item("Quit openXplane", "Esc", Some("Quit"))]),
];

// The target is blended in linear space, so these are linear values of the sRGB colours noted.
// bar sRGB (43, 46, 51)
const BAR: Color = [0.024, 0.027, 0.033, 0.97];
// drop-down sRGB (38, 41, 46)
const DROP: Color = [0.019, 0.022, 0.027, 0.97];
const EDGE: Color = [0.147, 0.173, 0.217, 1.0];
const TEXT: Color = [0.85, 0.87, 0.9, 1.0];
const DIM: Color = [0.25, 0.27, 0.3, 1.0];
const HOT: Color = [0.01, 0.1, 0.46, 1.0];

#[derive(Default)]
pub struct Menu {
    pub open: Option<usize>,
    pointer: Option<(f32, f32)>,
}

struct Layout {
    scale: f32,
    text: f32,
    bar_h: f32,
    row_h: f32,
    pad: f32,
}

fn layout(height: f32) -> Layout {
    let scale = (height / 800.0).clamp(0.7, 1.6);
    let text = 0.75 * scale;
    Layout {
        scale,
        text,
        bar_h: 34.0 * scale,
        row_h: 30.0 * scale,
        pad: 14.0 * scale,
    }
}

/// Left edge and width of every title on the bar.
fn titles(l: &Layout) -> Vec<(f32, f32)> {
    let mut x = 0.0;
    MENUS
        .iter()
        .map(|(name, _)| {
            let w = Hud::text_width(name, l.text) + 2.0 * l.pad;
            let r = (x, w);
            x += w;
            r
        })
        .collect()
}

/// Left edge, width and top of a drop-down.
fn drop_box(l: &Layout, index: usize) -> (f32, f32, f32) {
    let (x, _) = titles(l)[index];
    let items = MENUS[index].1;
    let widest = items
        .iter()
        .map(|i| {
            Hud::text_width(i.label, l.text)
                + Hud::text_width(i.shortcut, l.text)
                + if i.shortcut.is_empty() {
                    0.0
                } else {
                    3.0 * l.pad
                }
        })
        .fold(0.0, f32::max);
    (x, widest + 2.0 * l.pad, l.bar_h)
}

/// What a click did.
#[derive(Debug, PartialEq)]
pub enum Click {
    /// Not over the menu: the click belongs to the scene.
    Outside,
    /// Handled by the menu (opened, closed or hit a dimmed item).
    Consumed,
    /// An item with this key name was chosen.
    Run(&'static str),
}

impl Menu {
    /// A menu with drop-down `index` open (for screenshots).
    pub fn opened(index: usize) -> Self {
        Self {
            open: Some(index),
            pointer: None,
        }
    }

    pub fn visible(&self, height: f32) -> bool {
        self.open.is_some() || self.pointer.is_some_and(|(_, y)| y < layout(height).bar_h)
    }

    pub fn pointer_left(&mut self) {
        self.pointer = None;
    }

    pub fn pointer_moved(&mut self, x: f32, y: f32, height: f32) {
        self.pointer = Some((x, y));
        let l = layout(height);
        // moving along the bar while a menu is open switches menus
        if self.open.is_some()
            && y < l.bar_h
            && let Some(i) = titles(&l).iter().position(|&(tx, w)| x >= tx && x < tx + w)
        {
            self.open = Some(i);
        }
    }

    /// Whether the pointer is over the bar or an open drop-down (drags must not move the camera then).
    pub fn covers(&self, x: f32, y: f32, height: f32) -> bool {
        let l = layout(height);
        if y < l.bar_h && self.visible(height) {
            return true;
        }
        self.open.is_some_and(|i| {
            let (bx, bw, top) = drop_box(&l, i);
            let bh = l.row_h * MENUS[i].1.len() as f32;
            x >= bx && x < bx + bw && y >= top && y < top + bh
        })
    }

    pub fn click(&mut self, x: f32, y: f32, height: f32) -> Click {
        let l = layout(height);
        if y < l.bar_h && self.visible(height) {
            let hit = titles(&l).iter().position(|&(tx, w)| x >= tx && x < tx + w);
            self.open = match (hit, self.open) {
                (Some(i), Some(o)) if i == o => None,
                (hit, _) => hit,
            };
            return Click::Consumed;
        }
        let Some(open) = self.open else {
            return Click::Outside;
        };
        let (bx, bw, top) = drop_box(&l, open);
        let items = MENUS[open].1;
        let inside = x >= bx && x < bx + bw && y >= top;
        let row = ((y - top) / l.row_h) as usize;
        self.open = None;
        if inside && row < items.len() {
            return match items[row].key {
                Some(key) => Click::Run(key),
                None => Click::Consumed,
            };
        }
        // a click outside closes the menu without reaching the scene
        Click::Consumed
    }

    /// Draws the bar and the open drop-down. `hover` highlights an item (also used by screenshots).
    pub fn draw(&self, hud: &mut Hud, hover: Option<(usize, usize)>) {
        let (w, h) = hud.size();
        if !self.visible(h) {
            return;
        }
        let l = layout(h);
        hud.rect(0.0, 0.0, w, l.bar_h, BAR);
        hud.rect(0.0, l.bar_h - 1.0, w, 1.0, EDGE);
        let ty = (l.bar_h - 26.0 * l.text) * 0.5;
        let pointer = self.pointer;
        for (i, ((name, _), (x, tw))) in MENUS.iter().zip(titles(&l)).enumerate() {
            let over = pointer.is_some_and(|(px, py)| py < l.bar_h && px >= x && px < x + tw);
            if self.open == Some(i) || over {
                hud.rect(x, 0.0, tw, l.bar_h - 1.0, HOT);
            }
            hud.text(x + l.pad, ty, l.text, TEXT, name);
        }
        if let Some(open) = self.open {
            let (bx, bw, top) = drop_box(&l, open);
            let items = MENUS[open].1;
            let bh = l.row_h * items.len() as f32;
            hud.rect(bx - 1.0, top, bw + 2.0, bh + 1.0, EDGE);
            hud.rect(bx, top, bw, bh, DROP);
            for (r, it) in items.iter().enumerate() {
                let y = top + r as f32 * l.row_h;
                let over = hover == Some((open, r))
                    || pointer.is_some_and(|(px, py)| {
                        px >= bx && px < bx + bw && py >= y && py < y + l.row_h
                    });
                if over && it.key.is_some() {
                    hud.rect(bx, y, bw, l.row_h, HOT);
                }
                let color = if it.key.is_some() { TEXT } else { DIM };
                let iy = y + (l.row_h - 26.0 * l.text) * 0.5;
                hud.text(bx + l.pad, iy, l.text, color, it.label);
                if !it.shortcut.is_empty() {
                    let sw = Hud::text_width(it.shortcut, l.text);
                    hud.text(bx + bw - l.pad - sw, iy, l.text, DIM, it.shortcut);
                }
            }
        }
        let _ = l.scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_shows_near_the_top_and_opens_menus() {
        let mut m = Menu::default();
        assert!(!m.visible(800.0));
        m.pointer_moved(10.0, 5.0, 800.0);
        assert!(m.visible(800.0));
        assert_eq!(m.click(10.0, 5.0, 800.0), Click::Consumed);
        assert_eq!(m.open, Some(0));
        // the first drop-down row is "Keyboard help"
        let l = layout(800.0);
        assert_eq!(
            m.click(20.0, l.bar_h + l.row_h * 0.5, 800.0),
            Click::Run("H")
        );
        assert_eq!(m.open, None);
    }

    #[test]
    fn dimmed_items_and_outside_clicks() {
        let mut m = Menu::default();
        m.pointer_moved(10.0, 5.0, 800.0);
        m.click(10.0, 5.0, 800.0);
        let l = layout(800.0);
        assert_eq!(
            m.click(20.0, l.bar_h + l.row_h * 1.5, 800.0),
            Click::Consumed
        );
        // closed menu: clicks belong to the scene
        assert_eq!(m.click(400.0, 400.0, 800.0), Click::Outside);
        m.pointer_moved(10.0, 5.0, 800.0);
        m.click(10.0, 5.0, 800.0);
        assert!(m.covers(20.0, l.bar_h + 2.0, 800.0));
        assert_eq!(m.click(700.0, 600.0, 800.0), Click::Consumed);
        assert_eq!(m.open, None);
    }

    #[test]
    fn moving_along_the_bar_switches_the_open_menu() {
        let mut m = Menu::default();
        m.pointer_moved(5.0, 5.0, 800.0);
        m.click(5.0, 5.0, 800.0);
        let l = layout(800.0);
        let (x, w) = titles(&l)[3];
        m.pointer_moved(x + w * 0.5, 5.0, 800.0);
        assert_eq!(m.open, Some(3));
    }
}
