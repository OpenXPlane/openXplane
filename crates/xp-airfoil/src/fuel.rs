//! The fuel tank bookkeeping of the reference build (`0x14117c380`), the object at `F+0xbd00`.

/// The fields of the tank object: the two layout flags at `+0x18` and `+0x1c`, the amounts drawn so far at
/// `+0x34`, `+0x38`, `+0x3c` and the capacities at `+0x40`, `+0x44`, `+0x48`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tanks {
    pub flags: [i32; 2],
    pub used: [f32; 3],
    pub capacity: [f32; 3],
}

impl Tanks {
    /// Draws `amount` of fuel over `interval` in the tank mode `mode`: 1 left, 3 right, 2 centre, 5 all three
    /// (the flags choose how many tanks the modes 1, 2 and 3 share between); any other mode draws nothing.
    /// Returns whether the tanks could supply it. The drawn amount is added to the tanks' used totals.
    pub fn draw(&mut self, amount: f32, interval: f32, mode: i32) -> bool {
        let [a, b] = self.flags;
        let [u34, u38, u3c] = self.used;
        let [c40, c44, c48] = self.capacity;
        let need = amount * interval;
        let all_three = |t: &mut Tanks| {
            let remaining = c44 + c40 + c48 - u38 - u34 - u3c;
            if remaining > need {
                let share = amount / 3.0;
                t.used = [u34 + share, u38 + share, u3c + share];
                true
            } else {
                false
            }
        };
        match mode {
            1 | 5 | 3
                if mode == 5
                    || (mode == 1 && a != 0 && b != 0)
                    || (mode == 3 && a != 0 && b != 0) =>
            {
                all_three(self)
            }
            1 => {
                if a != 0 {
                    if c40 + c44 - u38 - u34 > need {
                        let share = amount * 0.5;
                        self.used[0] = u34 + share;
                        self.used[1] = u38 + share;
                        return true;
                    }
                } else if c40 - u38 > need {
                    self.used[1] = u38 + amount;
                    return true;
                }
                false
            }
            3 => {
                if b != 0 {
                    if c48 + c44 - u3c - u34 > need {
                        let share = amount * 0.5;
                        self.used[0] = u34 + share;
                        self.used[2] = u3c + share;
                        return true;
                    }
                } else if c48 - u3c > need {
                    self.used[2] = u3c + amount;
                    return true;
                }
                false
            }
            2 => {
                if a != 0 {
                    if b != 0 {
                        all_three(self)
                    } else if c40 + c44 - u38 - u34 > need {
                        let share = amount * 0.5;
                        self.used[0] = u34 + share;
                        self.used[1] = u38 + share;
                        true
                    } else {
                        false
                    }
                } else if b != 0 {
                    if c44 + c48 - u3c - u34 > need {
                        let share = amount * 0.5;
                        self.used[0] = u34 + share;
                        self.used[2] = u3c + share;
                        true
                    } else {
                        false
                    }
                } else if c44 - u34 > need {
                    self.used[0] = u34 + amount;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
