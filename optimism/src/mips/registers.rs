use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

pub const REGISTER_HI: usize = 32;
pub const REGISTER_LO: usize = 33;
pub const REGISTER_CURRENT_IP: usize = 34;
pub const REGISTER_NEXT_IP: usize = 35;

pub const NUM_REGISTERS: usize = 36;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Registers<T> {
    pub general_purpose: [T; 32],
    pub hi: T,
    pub lo: T,
    pub current_instruction_pointer: T,
    pub next_instruction_pointer: T,
}

impl<T> Registers<T> {
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.general_purpose.iter().chain([
            &self.hi,
            &self.lo,
            &self.current_instruction_pointer,
            &self.next_instruction_pointer,
        ])
    }
}

impl<T: Clone> Index<usize> for Registers<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index < 32 {
            &self.general_purpose[index]
        } else if index == REGISTER_HI {
            &self.hi
        } else if index == REGISTER_LO {
            &self.lo
        } else if index == REGISTER_CURRENT_IP {
            &self.current_instruction_pointer
        } else if index == REGISTER_NEXT_IP {
            &self.next_instruction_pointer
        } else {
            panic!("Index out of bounds");
        }
    }
}

impl<T: Clone> IndexMut<usize> for Registers<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < 32 {
            &mut self.general_purpose[index]
        } else if index == REGISTER_HI {
            &mut self.hi
        } else if index == REGISTER_LO {
            &mut self.lo
        } else if index == REGISTER_CURRENT_IP {
            &mut self.current_instruction_pointer
        } else if index == REGISTER_NEXT_IP {
            &mut self.next_instruction_pointer
        } else {
            panic!("Index out of bounds");
        }
    }
}
