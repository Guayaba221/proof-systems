//! This module defines the custom columns used in the Keccak witness, which
//! are aliases for the actual Keccak witness columns also defined here.
use crate::keccak::{ZKVM_KECCAK_COLS_CURR, ZKVM_KECCAK_COLS_NEXT};
use kimchi::circuits::polynomials::keccak::constants::{
    CHI_SHIFTS_B_LEN, CHI_SHIFTS_B_OFF, CHI_SHIFTS_SUM_LEN, CHI_SHIFTS_SUM_OFF, PIRHO_DENSE_E_LEN,
    PIRHO_DENSE_E_OFF, PIRHO_DENSE_ROT_E_LEN, PIRHO_DENSE_ROT_E_OFF, PIRHO_EXPAND_ROT_E_LEN,
    PIRHO_EXPAND_ROT_E_OFF, PIRHO_QUOTIENT_E_LEN, PIRHO_QUOTIENT_E_OFF, PIRHO_REMAINDER_E_LEN,
    PIRHO_REMAINDER_E_OFF, PIRHO_SHIFTS_E_LEN, PIRHO_SHIFTS_E_OFF, QUARTERS, RATE_IN_BYTES,
    SPONGE_BYTES_LEN, SPONGE_BYTES_OFF, SPONGE_NEW_STATE_LEN, SPONGE_NEW_STATE_OFF,
    SPONGE_SHIFTS_LEN, SPONGE_SHIFTS_OFF, SPONGE_ZEROS_LEN, SPONGE_ZEROS_OFF, STATE_LEN,
    THETA_DENSE_C_LEN, THETA_DENSE_C_OFF, THETA_DENSE_ROT_C_LEN, THETA_DENSE_ROT_C_OFF,
    THETA_EXPAND_ROT_C_LEN, THETA_EXPAND_ROT_C_OFF, THETA_QUOTIENT_C_LEN, THETA_QUOTIENT_C_OFF,
    THETA_REMAINDER_C_LEN, THETA_REMAINDER_C_OFF, THETA_SHIFTS_C_LEN, THETA_SHIFTS_C_OFF,
};
use kimchi_msm::witness::Witness;
use std::ops::{Index, IndexMut};

/// The total number of witness columns used by the Keccak circuit.
pub const ZKVM_KECCAK_COLS: usize =
    ZKVM_KECCAK_COLS_CURR + ZKVM_KECCAK_COLS_NEXT + MODE_FLAGS_COLS_LEN + STATUS_FLAGS_LEN;

// The number of columns used by the Keccak circuit to represent the status flags.
const STATUS_FLAGS_LEN: usize = 3;
// The number of columns used by the Keccak circuit to represent the mode flags.
const MODE_FLAGS_COLS_LEN: usize = ROUND_COEFFS_OFF + ROUND_COEFFS_LEN;
const FLAG_ROUND_OFF: usize = 0; // Offset of the FlagRound column inside the mode flags
const FLAG_ABSORB_OFF: usize = 1; // Offset of the FlagAbsorb column inside the mode flags
const FLAG_SQUEEZE_OFF: usize = 2; // Offset of the FlagSqueeze column inside the mode flags
const FLAG_ROOT_OFF: usize = 3; // Offset of the FlagRoot column inside the mode flags
const PAD_BYTES_OFF: usize = 4; // Offset of the PadBytesFlags inside the sponge coefficients
pub(crate) const PAD_BYTES_LEN: usize = RATE_IN_BYTES; // The maximum number of padding bytes involved
const PAD_LEN_OFF: usize = PAD_BYTES_OFF + PAD_BYTES_LEN; // Offset of the PadLength column inside the sponge coefficients
const PAD_INV_OFF: usize = PAD_LEN_OFF + 1; // Offset of the InvPadLength column inside the sponge coefficients
const PAD_TWO_OFF: usize = PAD_INV_OFF + 1; // Offset of the TwoToPad column inside the sponge coefficients
const PAD_SUFFIX_OFF: usize = PAD_TWO_OFF + 1; // Offset of the PadSuffix column inside the sponge coefficients
pub(crate) const PAD_SUFFIX_LEN: usize = 5; // The padding suffix of 1088 bits is stored as 5 field elements: 1x12 + 4x31 bytes
const ROUND_COEFFS_OFF: usize = PAD_SUFFIX_OFF + PAD_SUFFIX_LEN; // The round constants are located after the witness columns used by the Keccak round.
pub(crate) const ROUND_COEFFS_LEN: usize = QUARTERS; // The round constant of each round is stored in expanded form as quarters

/// Column aliases used by the Keccak circuit.
/// The number of aliases is not necessarily equal to the actual number of
/// columns.
/// Each alias will be mapped to a column index depending on the step kind
/// (Sponge or Round) that is currently being executed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Column {
    /// Hash identifier to distinguish inside the syscalls communication channel
    HashIndex,
    /// Block index inside the hash to enumerate preimage bytes
    BlockIndex,
    /// Hash step identifier to distinguish inside interstep communication
    StepIndex,
    /// Coeff Round = [0..24)
    FlagRound,
    FlagAbsorb,             // Coeff Absorb = 0 | 1
    FlagSqueeze,            // Coeff Squeeze = 0 | 1
    FlagRoot,               // Coeff Root = 0 | 1
    PadLength,              // Coeff Length 0 | 1 ..=136
    InvPadLength,           // Inverse of PadLength when PadLength != 0
    TwoToPad,               // 2^PadLength
    PadBytesFlags(usize),   // 136 boolean values
    PadSuffix(usize),       // 5 values with padding suffix
    RoundConstants(usize),  // Round constants
    Input(usize),           // Curr[0..100) either ThetaStateA or SpongeOldState
    ThetaShiftsC(usize),    // Round Curr[100..180)
    ThetaDenseC(usize),     // Round Curr[180..200)
    ThetaQuotientC(usize),  // Round Curr[200..205)
    ThetaRemainderC(usize), // Round Curr[205..225)
    ThetaDenseRotC(usize),  // Round Curr[225..245)
    ThetaExpandRotC(usize), // Round Curr[245..265)
    PiRhoShiftsE(usize),    // Round Curr[265..665)
    PiRhoDenseE(usize),     // Round Curr[665..765)
    PiRhoQuotientE(usize),  // Round Curr[765..865)
    PiRhoRemainderE(usize), // Round Curr[865..965)
    PiRhoDenseRotE(usize),  // Round Curr[965..1065)
    PiRhoExpandRotE(usize), // Round Curr[1065..1165)
    ChiShiftsB(usize),      // Round Curr[1165..1565)
    ChiShiftsSum(usize),    // Round Curr[1565..1965)
    SpongeNewState(usize),  // Sponge Curr[100..200)
    SpongeZeros(usize),     // Sponge Curr[168..200)
    SpongeBytes(usize),     // Sponge Curr[200..400)
    SpongeShifts(usize),    // Sponge Curr[400..800)
    Output(usize),          // Next[0..100) either IotaStateG or SpongeXorState
}

/// The witness columns used by the Keccak circuit.
/// The Keccak circuit is split into two main modes: Sponge and Round.
/// The columns are shared between the Sponge and Round steps.
/// The hash and step indices are shared between both modes.
/// The row is split into the following entries:
/// - hash_index: Which hash this is inside the circuit
/// - step_index: Which step this is inside the hash
/// - mode_flags: Round, Absorb, Squeeze, Root, PadLength, InvPadLength, TwoToPad, PadBytesFlags, PadSuffix, RoundConstants
/// - curr: Contains 1969 witnesses used in the current step including Input
/// - next: Contains the Output
///
///   Keccak Witness Columns: KeccakWitness.cols
///  ----------------------------------------------
/// | 0 | 1 | 2 | 3..154 | 155..2119 | 2120..2219 |
///  ----------------------------------------------
///   0 -> hash_index
///   1 -> block_index
///   2 -> step_index
///   3..154 -> mode_flags
///          -> 3: FlagRound
///          -> 4: FlagAbsorb
///          -> 5: FlagSqueeze
///          -> 6: FlagRoot
///          -> 7..142: PadBytesFlags
///          -> 143: PadLength
///          -> 144: InvPadLength
///          -> 145: TwoToPad
///          -> 146..150: PadSuffix
///          -> 151..154: RoundConstants
///   155..2123 -> curr
///         155                                                                        2119
///          <--------------------------------if_round<---------------------------------->
///          <-------------if_sponge------------->
///         155                                 954
///          -> SPONGE:                          | -> ROUND:
///         -> 155..254: Input == SpongeOldState | -> 155..254: Input == ThetaStateA
///         -> 255..354: SpongeNewState          | -> 255..334: ThetaShiftsC
///                    : 323..354 -> SpongeZeros | -> 335..354: ThetaDenseC
///         -> 355..554: SpongeBytes             | -> 355..359: ThetaQuotientC
///         -> 555..954: SpongeShifts            | -> 360..379: ThetaRemainderC
///                                              | -> 380..399: ThetaDenseRotC
///                                              | -> 400..419: ThetaExpandRotC
///                                              | -> 420..819: PiRhoShiftsE
///                                              | -> 820..919: PiRhoDenseE
///                                              | -> 920..1019: PiRhoQuotientE
///                                              | -> 1020..1119: PiRhoRemainderE
///                                              | -> 1120..1219: PiRhoDenseRotE
///                                              | -> 1220..1319: PiRhoExpandRotE
///                                              | -> 1320..1719: ChiShiftsB
///                                              | -> 1720..2119: ChiShiftsSum
///   2119..2219 -> next
///        -> 2124..2219: Output (if Round, then IotaStateG, if Sponge then SpongeXorState)
///
pub type KeccakWitness<T> = Witness<ZKVM_KECCAK_COLS, T>;

pub trait KeccakWitnessTrait<T> {
    /// Returns the hash index
    fn hash_index(&self) -> &T;
    /// Returns the block index
    fn block_index(&self) -> &T;
    /// Returns the step index
    fn step_index(&self) -> &T;
    /// Returns the mode flags
    fn mode_flags(&self) -> &[T];
    /// Returns the mode flags as a mutable reference
    fn mode_flags_mut(&mut self) -> &mut [T];
    /// Returns the `curr` witness columns
    fn curr(&self) -> &[T];
    /// Returns the `curr` witness columns as a mutable reference
    fn curr_mut(&mut self) -> &mut [T];
    /// Returns the `next` witness columns
    fn next(&self) -> &[T];
    /// Returns the `next` witness columns as a mutable reference
    fn next_mut(&mut self) -> &mut [T];
    /// Returns a chunk of the `curr` witness columns
    fn chunk(&self, offset: usize, length: usize) -> &[T];
}

impl<T: Clone> KeccakWitnessTrait<T> for KeccakWitness<T> {
    fn hash_index(&self) -> &T {
        &self.cols[0]
    }

    fn block_index(&self) -> &T {
        &self.cols[1]
    }

    fn step_index(&self) -> &T {
        &self.cols[2]
    }

    fn mode_flags(&self) -> &[T] {
        &self.cols[STATUS_FLAGS_LEN..STATUS_FLAGS_LEN + MODE_FLAGS_COLS_LEN]
    }

    fn mode_flags_mut(&mut self) -> &mut [T] {
        &mut self.cols[STATUS_FLAGS_LEN..STATUS_FLAGS_LEN + MODE_FLAGS_COLS_LEN]
    }

    fn curr(&self) -> &[T] {
        &self.cols[STATUS_FLAGS_LEN + MODE_FLAGS_COLS_LEN
            ..STATUS_FLAGS_LEN + MODE_FLAGS_COLS_LEN + ZKVM_KECCAK_COLS_CURR]
    }

    fn curr_mut(&mut self) -> &mut [T] {
        &mut self.cols[STATUS_FLAGS_LEN + MODE_FLAGS_COLS_LEN
            ..STATUS_FLAGS_LEN + MODE_FLAGS_COLS_LEN + ZKVM_KECCAK_COLS_CURR]
    }

    fn next(&self) -> &[T] {
        &self.cols[STATUS_FLAGS_LEN + MODE_FLAGS_COLS_LEN + ZKVM_KECCAK_COLS_CURR..]
    }

    fn next_mut(&mut self) -> &mut [T] {
        &mut self.cols[STATUS_FLAGS_LEN + MODE_FLAGS_COLS_LEN + ZKVM_KECCAK_COLS_CURR..]
    }

    fn chunk(&self, offset: usize, length: usize) -> &[T] {
        &self.curr()[offset..offset + length]
    }
}

impl<T: Clone> Index<Column> for KeccakWitness<T> {
    type Output = T;

    /// Map the column alias to the actual column index.
    /// Note that the column index depends on the step kind (Sponge or Round).
    /// For instance, the column 800 represents PadLength in the Sponge step, while it
    /// is used by intermediary values when executing the Round step.
    fn index(&self, index: Column) -> &Self::Output {
        match index {
            Column::HashIndex => self.hash_index(),
            Column::BlockIndex => self.block_index(),
            Column::StepIndex => self.step_index(),
            Column::FlagRound => &self.mode_flags()[FLAG_ROUND_OFF],
            Column::FlagAbsorb => &self.mode_flags()[FLAG_ABSORB_OFF],
            Column::FlagSqueeze => &self.mode_flags()[FLAG_SQUEEZE_OFF],
            Column::FlagRoot => &self.mode_flags()[FLAG_ROOT_OFF],
            Column::PadLength => &self.mode_flags()[PAD_LEN_OFF],
            Column::InvPadLength => &self.mode_flags()[PAD_INV_OFF],
            Column::TwoToPad => &self.mode_flags()[PAD_TWO_OFF],
            Column::PadBytesFlags(idx) => {
                assert!(idx < PAD_BYTES_LEN);
                &self.mode_flags()[PAD_BYTES_OFF + idx]
            }
            Column::PadSuffix(idx) => {
                assert!(idx < PAD_SUFFIX_LEN);
                &self.mode_flags()[PAD_SUFFIX_OFF + idx]
            }
            Column::RoundConstants(idx) => {
                assert!(idx < ROUND_COEFFS_LEN);
                &self.mode_flags()[ROUND_COEFFS_OFF + idx]
            }
            Column::Input(idx) => {
                assert!(idx < STATE_LEN);
                &self.curr()[idx]
            }
            Column::ThetaShiftsC(idx) => {
                assert!(idx < THETA_SHIFTS_C_LEN);
                &self.curr()[THETA_SHIFTS_C_OFF + idx]
            }
            Column::ThetaDenseC(idx) => {
                assert!(idx < THETA_DENSE_C_LEN);
                &self.curr()[THETA_DENSE_C_OFF + idx]
            }
            Column::ThetaQuotientC(idx) => {
                assert!(idx < THETA_QUOTIENT_C_LEN);
                &self.curr()[THETA_QUOTIENT_C_OFF + idx]
            }
            Column::ThetaRemainderC(idx) => {
                assert!(idx < THETA_REMAINDER_C_LEN);
                &self.curr()[THETA_REMAINDER_C_OFF + idx]
            }
            Column::ThetaDenseRotC(idx) => {
                assert!(idx < THETA_DENSE_ROT_C_LEN);
                &self.curr()[THETA_DENSE_ROT_C_OFF + idx]
            }
            Column::ThetaExpandRotC(idx) => {
                assert!(idx < THETA_EXPAND_ROT_C_LEN);
                &self.curr()[THETA_EXPAND_ROT_C_OFF + idx]
            }
            Column::PiRhoShiftsE(idx) => {
                assert!(idx < PIRHO_SHIFTS_E_LEN);
                &self.curr()[PIRHO_SHIFTS_E_OFF + idx]
            }
            Column::PiRhoDenseE(idx) => {
                assert!(idx < PIRHO_DENSE_E_LEN);
                &self.curr()[PIRHO_DENSE_E_OFF + idx]
            }
            Column::PiRhoQuotientE(idx) => {
                assert!(idx < PIRHO_QUOTIENT_E_LEN);
                &self.curr()[PIRHO_QUOTIENT_E_OFF + idx]
            }
            Column::PiRhoRemainderE(idx) => {
                assert!(idx < PIRHO_REMAINDER_E_LEN);
                &self.curr()[PIRHO_REMAINDER_E_OFF + idx]
            }
            Column::PiRhoDenseRotE(idx) => {
                assert!(idx < PIRHO_DENSE_ROT_E_LEN);
                &self.curr()[PIRHO_DENSE_ROT_E_OFF + idx]
            }
            Column::PiRhoExpandRotE(idx) => {
                assert!(idx < PIRHO_EXPAND_ROT_E_LEN);
                &self.curr()[PIRHO_EXPAND_ROT_E_OFF + idx]
            }
            Column::ChiShiftsB(idx) => {
                assert!(idx < CHI_SHIFTS_B_LEN);
                &self.curr()[CHI_SHIFTS_B_OFF + idx]
            }
            Column::ChiShiftsSum(idx) => {
                assert!(idx < CHI_SHIFTS_SUM_LEN);
                &self.curr()[CHI_SHIFTS_SUM_OFF + idx]
            }
            Column::SpongeNewState(idx) => {
                assert!(idx < SPONGE_NEW_STATE_LEN);
                &self.curr()[SPONGE_NEW_STATE_OFF + idx]
            }
            Column::SpongeZeros(idx) => {
                assert!(idx < SPONGE_ZEROS_LEN);
                &self.curr()[SPONGE_ZEROS_OFF + idx]
            }
            Column::SpongeBytes(idx) => {
                assert!(idx < SPONGE_BYTES_LEN);
                &self.curr()[SPONGE_BYTES_OFF + idx]
            }
            Column::SpongeShifts(idx) => {
                assert!(idx < SPONGE_SHIFTS_LEN);
                &self.curr()[SPONGE_SHIFTS_OFF + idx]
            }
            Column::Output(idx) => {
                assert!(idx < STATE_LEN);
                &self.next()[idx]
            }
        }
    }
}

impl<T: Clone> IndexMut<Column> for KeccakWitness<T> {
    fn index_mut(&mut self, index: Column) -> &mut Self::Output {
        match index {
            Column::HashIndex => &mut self.cols[0],
            Column::BlockIndex => &mut self.cols[1],
            Column::StepIndex => &mut self.cols[2],
            Column::FlagRound => &mut self.mode_flags_mut()[FLAG_ROUND_OFF],
            Column::FlagAbsorb => &mut self.mode_flags_mut()[FLAG_ABSORB_OFF],
            Column::FlagSqueeze => &mut self.mode_flags_mut()[FLAG_SQUEEZE_OFF],
            Column::FlagRoot => &mut self.mode_flags_mut()[FLAG_ROOT_OFF],
            Column::PadLength => &mut self.mode_flags_mut()[PAD_LEN_OFF],
            Column::InvPadLength => &mut self.mode_flags_mut()[PAD_INV_OFF],
            Column::TwoToPad => &mut self.mode_flags_mut()[PAD_TWO_OFF],
            Column::PadBytesFlags(idx) => {
                assert!(idx < PAD_BYTES_LEN);
                &mut self.mode_flags_mut()[PAD_BYTES_OFF + idx]
            }
            Column::PadSuffix(idx) => {
                assert!(idx < PAD_SUFFIX_LEN);
                &mut self.mode_flags_mut()[PAD_SUFFIX_OFF + idx]
            }
            Column::RoundConstants(idx) => {
                assert!(idx < ROUND_COEFFS_LEN);
                &mut self.mode_flags_mut()[ROUND_COEFFS_OFF + idx]
            }
            Column::Input(idx) => {
                assert!(idx < STATE_LEN);
                &mut self.curr_mut()[idx]
            }
            Column::ThetaShiftsC(idx) => {
                assert!(idx < THETA_SHIFTS_C_LEN);
                &mut self.curr_mut()[THETA_SHIFTS_C_OFF + idx]
            }
            Column::ThetaDenseC(idx) => {
                assert!(idx < THETA_DENSE_C_LEN);
                &mut self.curr_mut()[THETA_DENSE_C_OFF + idx]
            }
            Column::ThetaQuotientC(idx) => {
                assert!(idx < THETA_QUOTIENT_C_LEN);
                &mut self.curr_mut()[THETA_QUOTIENT_C_OFF + idx]
            }
            Column::ThetaRemainderC(idx) => {
                assert!(idx < THETA_REMAINDER_C_LEN);
                &mut self.curr_mut()[THETA_REMAINDER_C_OFF + idx]
            }
            Column::ThetaDenseRotC(idx) => {
                assert!(idx < THETA_DENSE_ROT_C_LEN);
                &mut self.curr_mut()[THETA_DENSE_ROT_C_OFF + idx]
            }
            Column::ThetaExpandRotC(idx) => {
                assert!(idx < THETA_EXPAND_ROT_C_LEN);
                &mut self.curr_mut()[THETA_EXPAND_ROT_C_OFF + idx]
            }
            Column::PiRhoShiftsE(idx) => {
                assert!(idx < PIRHO_SHIFTS_E_LEN);
                &mut self.curr_mut()[PIRHO_SHIFTS_E_OFF + idx]
            }
            Column::PiRhoDenseE(idx) => {
                assert!(idx < PIRHO_DENSE_E_LEN);
                &mut self.curr_mut()[PIRHO_DENSE_E_OFF + idx]
            }
            Column::PiRhoQuotientE(idx) => {
                assert!(idx < PIRHO_QUOTIENT_E_LEN);
                &mut self.curr_mut()[PIRHO_QUOTIENT_E_OFF + idx]
            }
            Column::PiRhoRemainderE(idx) => {
                assert!(idx < PIRHO_REMAINDER_E_LEN);
                &mut self.curr_mut()[PIRHO_REMAINDER_E_OFF + idx]
            }
            Column::PiRhoDenseRotE(idx) => {
                assert!(idx < PIRHO_DENSE_ROT_E_LEN);
                &mut self.curr_mut()[PIRHO_DENSE_ROT_E_OFF + idx]
            }
            Column::PiRhoExpandRotE(idx) => {
                assert!(idx < PIRHO_EXPAND_ROT_E_LEN);
                &mut self.curr_mut()[PIRHO_EXPAND_ROT_E_OFF + idx]
            }
            Column::ChiShiftsB(idx) => {
                assert!(idx < CHI_SHIFTS_B_LEN);
                &mut self.curr_mut()[CHI_SHIFTS_B_OFF + idx]
            }
            Column::ChiShiftsSum(idx) => {
                assert!(idx < CHI_SHIFTS_SUM_LEN);
                &mut self.curr_mut()[CHI_SHIFTS_SUM_OFF + idx]
            }
            Column::SpongeNewState(idx) => {
                assert!(idx < SPONGE_NEW_STATE_LEN);
                &mut self.curr_mut()[SPONGE_NEW_STATE_OFF + idx]
            }
            Column::SpongeZeros(idx) => {
                assert!(idx < SPONGE_ZEROS_LEN);
                &mut self.curr_mut()[SPONGE_ZEROS_OFF + idx]
            }
            Column::SpongeBytes(idx) => {
                assert!(idx < SPONGE_BYTES_LEN);
                &mut self.curr_mut()[SPONGE_BYTES_OFF + idx]
            }
            Column::SpongeShifts(idx) => {
                assert!(idx < SPONGE_SHIFTS_LEN);
                &mut self.curr_mut()[SPONGE_SHIFTS_OFF + idx]
            }
            Column::Output(idx) => {
                assert!(idx < STATE_LEN);
                &mut self.next_mut()[idx]
            }
        }
    }
}
