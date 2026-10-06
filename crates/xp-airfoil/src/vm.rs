//! A sparse view of the original process's memory for ports that follow pointers the way the original does.
//!
//! Objects are placed at the same absolute addresses the emulator used when the vectors were recorded, so a pointer
//! stored in a word is directly usable. Words that were never written read as zero (the emulator's memory starts
//! zeroed), and only the words an original function read or wrote are present.
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Vm {
    words: HashMap<u64, u32>,
}

impl Vm {
    pub fn from_words(words: HashMap<u64, u32>) -> Self {
        Vm { words }
    }

    pub fn words(&self) -> &HashMap<u64, u32> {
        &self.words
    }

    fn word(&self, address: u64) -> u32 {
        self.words.get(&address).copied().unwrap_or(0)
    }

    pub fn u32(&self, address: u64) -> u32 {
        let shift = (address & 3) * 8;
        if shift == 0 {
            return self.word(address);
        }
        let base = address & !3;
        let low = u64::from(self.word(base));
        let high = u64::from(self.word(base + 4));
        ((low | high << 32) >> shift) as u32
    }

    pub fn set_u32(&mut self, address: u64, value: u32) {
        if address & 3 == 0 {
            self.words.insert(address, value);
            return;
        }
        let base = address & !3;
        let shift = (address & 3) * 8;
        let combined = u64::from(self.word(base)) | u64::from(self.word(base + 4)) << 32;
        let mask = 0xffff_ffffu64 << shift;
        let updated = (combined & !mask) | (u64::from(value) << shift);
        self.words.insert(base, updated as u32);
        self.words.insert(base + 4, (updated >> 32) as u32);
    }

    pub fn i32(&self, address: u64) -> i32 {
        self.u32(address) as i32
    }
    pub fn set_i32(&mut self, address: u64, value: i32) {
        self.set_u32(address, value as u32);
    }
    pub fn f32(&self, address: u64) -> f32 {
        f32::from_bits(self.u32(address))
    }
    pub fn set_f32(&mut self, address: u64, value: f32) {
        self.set_u32(address, value.to_bits());
    }
    pub fn u64(&self, address: u64) -> u64 {
        u64::from(self.u32(address)) | u64::from(self.u32(address + 4)) << 32
    }
    pub fn set_u64(&mut self, address: u64, value: u64) {
        self.set_u32(address, value as u32);
        self.set_u32(address + 4, (value >> 32) as u32);
    }
    pub fn f64(&self, address: u64) -> f64 {
        f64::from_bits(self.u64(address))
    }
    pub fn set_f64(&mut self, address: u64, value: f64) {
        self.set_u64(address, value.to_bits());
    }
    pub fn u8(&self, address: u64) -> u8 {
        (self.word(address & !3) >> ((address & 3) * 8)) as u8
    }
    pub fn set_u8(&mut self, address: u64, value: u8) {
        let base = address & !3;
        let shift = (address & 3) * 8;
        let updated = (self.word(base) & !(0xff << shift)) | (u32::from(value) << shift);
        self.words.insert(base, updated);
    }
}

impl crate::element_force::Mem for (&Vm, u64) {
    fn f32(&self, offset: usize) -> f32 {
        self.0.f32(self.1 + offset as u64)
    }
    fn i32(&self, offset: usize) -> i32 {
        self.0.i32(self.1 + offset as u64)
    }
}
