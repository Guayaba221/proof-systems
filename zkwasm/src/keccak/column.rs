use std::ops::{Index, IndexMut};

use ark_ff::{One, Zero};
use serde::{Deserialize, Serialize};

use super::grid_index;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum KeccakColumn {
    FlagRound,                                // Coeff Round = 0 | 1
    FlagAbsorb,                               // Coeff Absorb = 0 | 1
    FlagSqueeze,                              // Coeff Squeeze = 0 | 1
    FlagRoot,                                 // Coeff Root = 0 | 1
    FlagPad,                                  // Coeff Pad = 0 | 1
    FlagLength,                               // Coeff Length 0 | 1 .. 136
    TwoToPad,                                 // 2^PadLength
    FlagsBytes(usize),                        // 136 boolean values
    PadSuffix(usize),                         // 5 values with padding suffix
    RoundConstants(usize),                    // Round constants
    ThetaStateA(usize, usize, usize),         // Round Curr[0..100)
    ThetaShiftsC(usize, usize, usize),        // Round Curr[100..180)
    ThetaDenseC(usize, usize),                // Round Curr[180..200)
    ThetaQuotientC(usize),                    // Round Curr[200..205)
    ThetaRemainderC(usize, usize),            // Round Curr[205..225)
    ThetaDenseRotC(usize, usize),             // Round Curr[225..245)
    ThetaExpandRotC(usize, usize),            // Round Curr[245..265)
    PiRhoShiftsE(usize, usize, usize, usize), // Round Curr[265..665)
    PiRhoDenseE(usize, usize, usize),         // Round Curr[665..765)
    PiRhoQuotientE(usize, usize, usize),      // Round Curr[765..865)
    PiRhoRemainderE(usize, usize, usize),     // Round Curr[865..965)
    PiRhoDenseRotE(usize, usize, usize),      // Round Curr[965..1065)
    PiRhoExpandRotE(usize, usize, usize),     // Round Curr[1065..1165)
    ChiShiftsB(usize, usize, usize, usize),   // Round Curr[1165..1565)
    ChiShiftsSum(usize, usize, usize, usize), // Round Curr[1565..1965)
    SpongeOldState(usize),                    // Sponge Curr[0..100)
    SpongeNewState(usize),                    // Sponge Curr[100..200)
    SpongeBytes(usize),                       // Sponge Curr[200..400)
    SpongeShifts(usize),                      // Sponge Curr[400..800)
    NextState(usize),                         // Sponge Next[0..100)
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct KeccakColumns<T> {
    pub flag_round: T,               // Coeff Round = [0..24)
    pub flag_absorb: T,              // Coeff Absorb = 0 | 1
    pub flag_squeeze: T,             // Coeff Squeeze = 0 | 1
    pub flag_root: T,                // Coeff Root = 0 | 1
    pub flag_pad: T,                 // Coeff Pad = 0 | 1
    pub flag_length: T,              // Coeff Length 0 | 1 .. 136
    pub two_to_pad: T,               // 2^PadLength
    pub flags_bytes: Vec<T>,         // 136 boolean values
    pub pad_suffix: Vec<T>,          // 5 values with padding suffix
    pub round_constants: Vec<T>,     // Round constants
    pub theta_state_a: Vec<T>,       // Round Curr[0..100)
    pub theta_shifts_c: Vec<T>,      // Round Curr[100..180)
    pub theta_dense_c: Vec<T>,       // Round Curr[180..200)
    pub theta_quotient_c: Vec<T>,    // Round Curr[200..205)
    pub theta_remainder_c: Vec<T>,   // Round Curr[205..225)
    pub theta_dense_rot_c: Vec<T>,   // Round Curr[225..245)
    pub theta_expand_rot_c: Vec<T>,  // Round Curr[245..265)
    pub pi_rho_shifts_e: Vec<T>,     // Round Curr[265..665)
    pub pi_rho_dense_e: Vec<T>,      // Round Curr[665..765)
    pub pi_rho_quotient_e: Vec<T>,   // Round Curr[765..865)
    pub pi_rho_remainder_e: Vec<T>,  // Round Curr[865..965)
    pub pi_rho_dense_rot_e: Vec<T>,  // Round Curr[965..1065)
    pub pi_rho_expand_rot_e: Vec<T>, // Round Curr[1065..1165)
    pub chi_shifts_b: Vec<T>,        // Round Curr[1165..1565)
    pub chi_shifts_sum: Vec<T>,      // Round Curr[1565..1965)
    pub sponge_old_state: Vec<T>,    // Sponge Curr[0..100)
    pub sponge_new_state: Vec<T>,    // Sponge Curr[100..200)
    pub sponge_bytes: Vec<T>,        // Sponge Curr[200..400)
    pub sponge_shifts: Vec<T>,       // Sponge Curr[400..800)
    pub next_state: Vec<T>,          // Sponge Next[0..100)
}

impl<T: Zero + One + Clone> Default for KeccakColumns<T> {
    fn default() -> Self {
        KeccakColumns {
            flag_round: T::zero(),
            flag_absorb: T::zero(),
            flag_squeeze: T::zero(),
            flag_root: T::zero(),
            flag_pad: T::zero(),
            flag_length: T::zero(),
            two_to_pad: T::one(), // So that default 2^0 is in the table
            flags_bytes: vec![T::zero(); 136],
            pad_suffix: vec![T::zero(); 5],
            round_constants: vec![T::zero(); 4],
            theta_state_a: vec![T::zero(); 100],
            theta_shifts_c: vec![T::zero(); 80],
            theta_dense_c: vec![T::zero(); 20],
            theta_quotient_c: vec![T::zero(); 5],
            theta_remainder_c: vec![T::zero(); 20],
            theta_dense_rot_c: vec![T::zero(); 20],
            theta_expand_rot_c: vec![T::zero(); 20],
            pi_rho_shifts_e: vec![T::zero(); 400],
            pi_rho_dense_e: vec![T::zero(); 100],
            pi_rho_quotient_e: vec![T::zero(); 100],
            pi_rho_remainder_e: vec![T::zero(); 100],
            pi_rho_dense_rot_e: vec![T::zero(); 100],
            pi_rho_expand_rot_e: vec![T::zero(); 100],
            chi_shifts_b: vec![T::zero(); 400],
            chi_shifts_sum: vec![T::zero(); 400],
            sponge_old_state: vec![T::zero(); 100],
            sponge_new_state: vec![T::zero(); 100],
            sponge_bytes: vec![T::zero(); 200],
            sponge_shifts: vec![T::zero(); 400],
            next_state: vec![T::zero(); 100],
        }
    }
}

impl<A> Index<KeccakColumn> for KeccakColumns<A> {
    type Output = A;

    fn index(&self, index: KeccakColumn) -> &Self::Output {
        match index {
            KeccakColumn::FlagRound => &self.flag_round,
            KeccakColumn::FlagAbsorb => &self.flag_absorb,
            KeccakColumn::FlagSqueeze => &self.flag_squeeze,
            KeccakColumn::FlagRoot => &self.flag_root,
            KeccakColumn::FlagPad => &self.flag_pad,
            KeccakColumn::FlagLength => &self.flag_length,
            KeccakColumn::TwoToPad => &self.two_to_pad,
            KeccakColumn::FlagsBytes(i) => &self.flags_bytes[i],
            KeccakColumn::PadSuffix(i) => &self.pad_suffix[i],
            KeccakColumn::RoundConstants(q) => &self.round_constants[q],
            KeccakColumn::ThetaStateA(y, x, q) => &self.theta_state_a[grid_index(100, 0, y, x, q)],
            KeccakColumn::ThetaShiftsC(i, x, q) => &self.theta_shifts_c[grid_index(80, i, 0, x, q)],
            KeccakColumn::ThetaDenseC(x, q) => &self.theta_dense_c[grid_index(20, 0, 0, x, q)],
            KeccakColumn::ThetaQuotientC(x) => &self.theta_quotient_c[x],
            KeccakColumn::ThetaRemainderC(x, q) => {
                &self.theta_remainder_c[grid_index(20, 0, 0, x, q)]
            }
            KeccakColumn::ThetaDenseRotC(x, q) => {
                &self.theta_dense_rot_c[grid_index(20, 0, 0, x, q)]
            }
            KeccakColumn::ThetaExpandRotC(x, q) => {
                &self.theta_expand_rot_c[grid_index(20, 0, 0, x, q)]
            }
            KeccakColumn::PiRhoShiftsE(i, y, x, q) => {
                &self.pi_rho_shifts_e[grid_index(400, i, y, x, q)]
            }
            KeccakColumn::PiRhoDenseE(y, x, q) => &self.pi_rho_dense_e[grid_index(100, 0, y, x, q)],
            KeccakColumn::PiRhoQuotientE(y, x, q) => {
                &self.pi_rho_quotient_e[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::PiRhoRemainderE(y, x, q) => {
                &self.pi_rho_remainder_e[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::PiRhoDenseRotE(y, x, q) => {
                &self.pi_rho_dense_rot_e[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::PiRhoExpandRotE(y, x, q) => {
                &self.pi_rho_expand_rot_e[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::ChiShiftsB(i, y, x, q) => &self.chi_shifts_b[grid_index(400, i, y, x, q)],
            KeccakColumn::ChiShiftsSum(i, y, x, q) => {
                &self.chi_shifts_sum[grid_index(400, i, y, x, q)]
            }
            KeccakColumn::SpongeOldState(i) => &self.sponge_old_state[i],
            KeccakColumn::SpongeNewState(i) => &self.sponge_new_state[i],
            KeccakColumn::SpongeBytes(i) => &self.sponge_bytes[i],
            KeccakColumn::SpongeShifts(i) => &self.sponge_shifts[i],
            KeccakColumn::NextState(i) => &self.next_state[i],
        }
    }
}

impl<A> IndexMut<KeccakColumn> for KeccakColumns<A> {
    fn index_mut(&mut self, index: KeccakColumn) -> &mut Self::Output {
        match index {
            KeccakColumn::FlagRound => &mut self.flag_round,
            KeccakColumn::FlagAbsorb => &mut self.flag_absorb,
            KeccakColumn::FlagSqueeze => &mut self.flag_squeeze,
            KeccakColumn::FlagRoot => &mut self.flag_root,
            KeccakColumn::FlagPad => &mut self.flag_pad,
            KeccakColumn::FlagLength => &mut self.flag_length,
            KeccakColumn::TwoToPad => &mut self.two_to_pad,
            KeccakColumn::FlagsBytes(i) => &mut self.flags_bytes[i],
            KeccakColumn::PadSuffix(i) => &mut self.pad_suffix[i],
            KeccakColumn::RoundConstants(q) => &mut self.round_constants[q],
            KeccakColumn::ThetaStateA(y, x, q) => {
                &mut self.theta_state_a[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::ThetaShiftsC(i, x, q) => {
                &mut self.theta_shifts_c[grid_index(80, i, 0, x, q)]
            }
            KeccakColumn::ThetaDenseC(x, q) => &mut self.theta_dense_c[grid_index(20, 0, 0, x, q)],
            KeccakColumn::ThetaQuotientC(x) => &mut self.theta_quotient_c[x],
            KeccakColumn::ThetaRemainderC(x, q) => {
                &mut self.theta_remainder_c[grid_index(20, 0, 0, x, q)]
            }
            KeccakColumn::ThetaDenseRotC(x, q) => {
                &mut self.theta_dense_rot_c[grid_index(20, 0, 0, x, q)]
            }
            KeccakColumn::ThetaExpandRotC(x, q) => {
                &mut self.theta_expand_rot_c[grid_index(20, 0, 0, x, q)]
            }
            KeccakColumn::PiRhoShiftsE(i, y, x, q) => {
                &mut self.pi_rho_shifts_e[grid_index(400, i, y, x, q)]
            }
            KeccakColumn::PiRhoDenseE(y, x, q) => {
                &mut self.pi_rho_dense_e[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::PiRhoQuotientE(y, x, q) => {
                &mut self.pi_rho_quotient_e[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::PiRhoRemainderE(y, x, q) => {
                &mut self.pi_rho_remainder_e[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::PiRhoDenseRotE(y, x, q) => {
                &mut self.pi_rho_dense_rot_e[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::PiRhoExpandRotE(y, x, q) => {
                &mut self.pi_rho_expand_rot_e[grid_index(100, 0, y, x, q)]
            }
            KeccakColumn::ChiShiftsB(i, y, x, q) => {
                &mut self.chi_shifts_b[grid_index(400, i, y, x, q)]
            }
            KeccakColumn::ChiShiftsSum(i, y, x, q) => {
                &mut self.chi_shifts_sum[grid_index(400, i, y, x, q)]
            }
            KeccakColumn::SpongeOldState(i) => &mut self.sponge_old_state[i],
            KeccakColumn::SpongeNewState(i) => &mut self.sponge_new_state[i],
            KeccakColumn::SpongeBytes(i) => &mut self.sponge_bytes[i],
            KeccakColumn::SpongeShifts(i) => &mut self.sponge_shifts[i],
            KeccakColumn::NextState(i) => &mut self.next_state[i],
        }
    }
}
