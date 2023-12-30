use super::{
    column::{KeccakColumn, KeccakColumns},
    interpreter::{Absorb, KeccakStep, Sponge},
    ArithOps, BoolOps, DIM, E, QUARTERS,
};
use crate::mips::interpreter::Lookup;
use ark_ff::{Field, One};
use kimchi::circuits::expr::Operations;
use kimchi::{
    auto_clone_array,
    circuits::{expr::ConstantTerm::Literal, polynomials::keccak::constants::ROUNDS},
    grid,
    o1_utils::Two,
};

#[derive(Clone, Debug)]
pub struct KeccakEnv<Fp> {
    /// Constraints that are added to the circuit
    pub(crate) constraints: Vec<E<Fp>>,
    /// Values that are looked up in the circuit
    pub(crate) lookups: Vec<Lookup<E<Fp>>>,
    /// Expanded block of previous step
    pub(crate) prev_block: Vec<u64>,
    /// Padded preimage data
    pub(crate) padded: Vec<u8>,
    /// Current block of preimage data
    pub(crate) block_idx: usize,
    /// The full state of the Keccak gate (witness)
    pub(crate) keccak_state: KeccakColumns<E<Fp>>,
    /// Byte-length of the 10*1 pad (<=136)
    pub(crate) pad_len: u64,
    /// How many blocks are left to absrob (including current absorb)
    pub(crate) blocks_left_to_absorb: u64,
    /// What step of the hash is being executed (or None, if just ended)
    pub(crate) curr_step: Option<KeccakStep>,
}

impl<Fp: Field> KeccakEnv<Fp> {
    pub fn write_column(&mut self, column: KeccakColumn, value: u64) {
        self.keccak_state[column] = Self::constant(value);
    }

    pub fn write_column_field(&mut self, column: KeccakColumn, value: Fp) {
        self.keccak_state[column] = Self::constant_field(value);
    }

    pub fn null_state(&mut self) {
        self.keccak_state = KeccakColumns::default();
    }
    pub fn update_step(&mut self) {
        match self.curr_step {
            Some(step) => match step {
                KeccakStep::Sponge(sponge) => match sponge {
                    Sponge::Absorb(_) => self.curr_step = Some(KeccakStep::Round(0)),
                    Sponge::Squeeze => self.curr_step = None,
                },
                KeccakStep::Round(round) => {
                    if round < ROUNDS as u64 - 1 {
                        self.curr_step = Some(KeccakStep::Round(round + 1));
                    } else {
                        self.blocks_left_to_absorb -= 1;
                        match self.blocks_left_to_absorb {
                            0 => self.curr_step = Some(KeccakStep::Sponge(Sponge::Squeeze)),
                            1 => {
                                self.curr_step =
                                    Some(KeccakStep::Sponge(Sponge::Absorb(Absorb::Last)))
                            }
                            _ => {
                                self.curr_step =
                                    Some(KeccakStep::Sponge(Sponge::Absorb(Absorb::Middle)))
                            }
                        }
                    }
                }
            },
            None => panic!("No step to update"),
        }
    }
}

impl<Fp: Field> BoolOps for KeccakEnv<Fp> {
    type Column = KeccakColumn;
    type Variable = E<Fp>;
    type Fp = Fp;

    fn boolean(x: Self::Variable) -> Self::Variable {
        x.clone() * (x - Self::Variable::one())
    }

    fn not(x: Self::Variable) -> Self::Variable {
        Self::Variable::one() - x
    }

    fn is_one(x: Self::Variable) -> Self::Variable {
        x - Self::Variable::one()
    }

    fn xor(x: Self::Variable, y: Self::Variable) -> Self::Variable {
        Self::is_one(x + y)
    }

    fn or(x: Self::Variable, y: Self::Variable) -> Self::Variable {
        x.clone() + y.clone() - x * y
    }

    fn either_false(x: Self::Variable, y: Self::Variable) -> Self::Variable {
        x * y
    }
}

impl<Fp: Field> ArithOps for KeccakEnv<Fp> {
    type Column = KeccakColumn;
    type Variable = E<Fp>;
    type Fp = Fp;
    fn constant(x: u64) -> Self::Variable {
        Self::constant_field(Self::Fp::from(x))
    }
    fn constant_field(x: Self::Fp) -> Self::Variable {
        Self::Variable::constant(Operations::from(Literal(x)))
    }
    fn zero() -> Self::Variable {
        Self::constant(0)
    }
    fn one() -> Self::Variable {
        Self::constant(1)
    }
    fn two() -> Self::Variable {
        Self::constant(2)
    }
    fn two_pow(x: u64) -> Self::Variable {
        Self::constant_field(Self::Fp::two_pow(x))
    }
}

pub(crate) trait KeccakEnvironment {
    type Column;
    type Variable: std::ops::Mul<Self::Variable, Output = Self::Variable>
        + std::ops::Add<Self::Variable, Output = Self::Variable>
        + std::ops::Sub<Self::Variable, Output = Self::Variable>
        + Clone;
    type Fp: std::ops::Neg<Output = Self::Fp>;

    fn from_shifts(
        shifts: &[Self::Variable],
        i: Option<usize>,
        y: Option<usize>,
        x: Option<usize>,
        q: Option<usize>,
    ) -> Self::Variable;

    fn from_quarters(quarters: &[Self::Variable], y: Option<usize>, x: usize) -> Self::Variable;

    fn is_sponge(&self) -> Self::Variable;

    fn is_round(&self) -> Self::Variable;

    fn round(&self) -> Self::Variable;

    fn absorb(&self) -> Self::Variable;

    fn squeeze(&self) -> Self::Variable;

    fn root(&self) -> Self::Variable;

    fn pad(&self) -> Self::Variable;

    fn length(&self) -> Self::Variable;

    fn two_to_pad(&self) -> Self::Variable;

    fn in_padding(&self, i: usize) -> Self::Variable;

    fn pad_suffix(&self, i: usize) -> Self::Variable;

    fn bytes_block(&self, i: usize) -> Vec<Self::Variable>;

    fn flags_block(&self, i: usize) -> Vec<Self::Variable>;

    fn block_in_padding(&self, i: usize) -> Self::Variable;

    fn round_constants(&self) -> Vec<Self::Variable>;

    fn old_state(&self, i: usize) -> Self::Variable;

    fn new_block(&self, i: usize) -> Self::Variable;

    fn next_state(&self, i: usize) -> Self::Variable;

    fn sponge_zeros(&self) -> Vec<Self::Variable>;

    fn sponge_shifts(&self) -> Vec<Self::Variable>;

    fn sponge_bytes(&self, i: usize) -> Self::Variable;

    fn state_a(&self, y: usize, x: usize, q: usize) -> Self::Variable;

    fn shifts_c(&self, i: usize, x: usize, q: usize) -> Self::Variable;

    fn dense_c(&self, x: usize, q: usize) -> Self::Variable;

    fn quotient_c(&self, x: usize) -> Self::Variable;

    fn remainder_c(&self, x: usize, q: usize) -> Self::Variable;

    fn dense_rot_c(&self, x: usize, q: usize) -> Self::Variable;

    fn expand_rot_c(&self, x: usize, q: usize) -> Self::Variable;

    fn shifts_e(&self, i: usize, y: usize, x: usize, q: usize) -> Self::Variable;

    fn dense_e(&self, y: usize, x: usize, q: usize) -> Self::Variable;

    fn quotient_e(&self, y: usize, x: usize, q: usize) -> Self::Variable;

    fn remainder_e(&self, y: usize, x: usize, q: usize) -> Self::Variable;

    fn dense_rot_e(&self, y: usize, x: usize, q: usize) -> Self::Variable;

    fn expand_rot_e(&self, y: usize, x: usize, q: usize) -> Self::Variable;

    fn shifts_b(&self, i: usize, y: usize, x: usize, q: usize) -> Self::Variable;

    fn shifts_sum(&self, i: usize, y: usize, x: usize, q: usize) -> Self::Variable;
}

impl<Fp: Field> KeccakEnvironment for KeccakEnv<Fp> {
    type Column = KeccakColumn;
    type Variable = E<Fp>;
    type Fp = Fp;

    fn from_shifts(
        shifts: &[Self::Variable],
        i: Option<usize>,
        y: Option<usize>,
        x: Option<usize>,
        q: Option<usize>,
    ) -> Self::Variable {
        match shifts.len() {
            400 => {
                if let Some(i) = i {
                    auto_clone_array!(shifts);
                    shifts(i)
                        + Self::two_pow(1) * shifts(100 + i)
                        + Self::two_pow(2) * shifts(200 + i)
                        + Self::two_pow(3) * shifts(300 + i)
                } else {
                    let shifts = grid!(400, shifts);
                    shifts(0, y.unwrap(), x.unwrap(), q.unwrap())
                        + Self::two_pow(1) * shifts(1, y.unwrap(), x.unwrap(), q.unwrap())
                        + Self::two_pow(2) * shifts(2, y.unwrap(), x.unwrap(), q.unwrap())
                        + Self::two_pow(3) * shifts(3, y.unwrap(), x.unwrap(), q.unwrap())
                }
            }
            100 => {
                let shifts = grid!(100, shifts);
                shifts(0, x.unwrap(), q.unwrap())
                    + Self::two_pow(1) * shifts(1, x.unwrap(), q.unwrap())
                    + Self::two_pow(2) * shifts(2, x.unwrap(), q.unwrap())
                    + Self::two_pow(3) * shifts(3, x.unwrap(), q.unwrap())
            }
            _ => panic!("Invalid length of shifts"),
        }
    }

    fn from_quarters(quarters: &[Self::Variable], y: Option<usize>, x: usize) -> Self::Variable {
        if let Some(y) = y {
            assert!(quarters.len() == 100, "Invalid length of quarters");
            let quarters = grid!(100, quarters);
            quarters(y, x, 0)
                + Self::two_pow(16) * quarters(y, x, 1)
                + Self::two_pow(32) * quarters(y, x, 2)
                + Self::two_pow(48) * quarters(y, x, 3)
        } else {
            assert!(quarters.len() == 20, "Invalid length of quarters");
            let quarters = grid!(20, quarters);
            quarters(x, 0)
                + Self::two_pow(16) * quarters(x, 1)
                + Self::two_pow(32) * quarters(x, 2)
                + Self::two_pow(48) * quarters(x, 3)
        }
    }

    fn is_sponge(&self) -> Self::Variable {
        Self::xor(self.absorb(), self.squeeze())
    }

    fn is_round(&self) -> Self::Variable {
        Self::not(self.is_sponge())
    }

    fn round(&self) -> Self::Variable {
        self.keccak_state[KeccakColumn::FlagRound].clone()
    }

    fn absorb(&self) -> Self::Variable {
        self.keccak_state[KeccakColumn::FlagAbsorb].clone()
    }

    fn squeeze(&self) -> Self::Variable {
        self.keccak_state[KeccakColumn::FlagSqueeze].clone()
    }

    fn root(&self) -> Self::Variable {
        self.keccak_state[KeccakColumn::FlagRoot].clone()
    }

    fn pad(&self) -> Self::Variable {
        self.keccak_state[KeccakColumn::FlagPad].clone()
    }

    fn length(&self) -> Self::Variable {
        self.keccak_state[KeccakColumn::FlagLength].clone()
    }

    fn two_to_pad(&self) -> Self::Variable {
        self.keccak_state[KeccakColumn::TwoToPad].clone()
    }

    fn in_padding(&self, i: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::FlagsBytes(i)].clone()
    }

    fn pad_suffix(&self, i: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::PadSuffix(i)].clone()
    }

    fn bytes_block(&self, i: usize) -> Vec<Self::Variable> {
        match i {
            0 => self.keccak_state.sponge_bytes[0..12].to_vec().clone(),
            1..=4 => self.keccak_state.sponge_bytes[12 + (i - 1) * 31..12 + i * 31]
                .to_vec()
                .clone(),
            _ => panic!("No more blocks of bytes can be part of padding"),
        }
    }

    fn flags_block(&self, i: usize) -> Vec<Self::Variable> {
        match i {
            0 => self.keccak_state.flags_bytes[0..12].to_vec().clone(),
            1..=4 => self.keccak_state.flags_bytes[12 + (i - 1) * 31..12 + i * 31]
                .to_vec()
                .clone(),
            _ => panic!("No more blocks of flags can be part of padding"),
        }
    }

    fn block_in_padding(&self, i: usize) -> Self::Variable {
        let bytes = self.bytes_block(i);
        let flags = self.flags_block(i);
        assert_eq!(bytes.len(), flags.len());
        let pad = bytes
            .iter()
            .zip(flags)
            .fold(Self::zero(), |acc, (byte, flag)| {
                acc + byte.clone() * flag * Self::two_pow(8)
            });

        pad
    }

    fn round_constants(&self) -> Vec<Self::Variable> {
        self.keccak_state.round_constants.clone()
    }

    fn old_state(&self, i: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::SpongeOldState(i)].clone()
    }

    fn new_block(&self, i: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::SpongeNewState(i)].clone()
    }

    fn next_state(&self, i: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::NextState(i)].clone()
    }

    fn sponge_zeros(&self) -> Vec<Self::Variable> {
        self.keccak_state.sponge_new_state[68..100].to_vec().clone()
    }

    fn sponge_bytes(&self, i: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::SpongeBytes(i)].clone()
    }

    fn sponge_shifts(&self) -> Vec<Self::Variable> {
        self.keccak_state.sponge_shifts.clone()
    }

    fn state_a(&self, x: usize, y: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::ThetaStateA(y, x, q)].clone()
    }

    fn shifts_c(&self, i: usize, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::ThetaShiftsC(i, x, q)].clone()
    }

    fn dense_c(&self, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::ThetaDenseC(x, q)].clone()
    }

    fn quotient_c(&self, x: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::ThetaQuotientC(x)].clone()
    }

    fn remainder_c(&self, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::ThetaRemainderC(x, q)].clone()
    }

    fn dense_rot_c(&self, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::ThetaDenseRotC(x, q)].clone()
    }

    fn expand_rot_c(&self, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::ThetaExpandRotC(x, q)].clone()
    }

    fn shifts_e(&self, i: usize, y: usize, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::PiRhoShiftsE(i, y, x, q)].clone()
    }

    fn dense_e(&self, y: usize, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::PiRhoDenseE(y, x, q)].clone()
    }

    fn quotient_e(&self, y: usize, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::PiRhoQuotientE(y, x, q)].clone()
    }

    fn remainder_e(&self, y: usize, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::PiRhoRemainderE(y, x, q)].clone()
    }

    fn dense_rot_e(&self, y: usize, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::PiRhoDenseRotE(y, x, q)].clone()
    }

    fn expand_rot_e(&self, y: usize, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::PiRhoExpandRotE(y, x, q)].clone()
    }

    fn shifts_b(&self, i: usize, y: usize, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::ChiShiftsB(i, y, x, q)].clone()
    }

    fn shifts_sum(&self, i: usize, y: usize, x: usize, q: usize) -> Self::Variable {
        self.keccak_state[KeccakColumn::ChiShiftsSum(i, y, x, q)].clone()
    }
}
