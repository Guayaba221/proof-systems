use super::{
    column::KeccakColumn,
    environment::{KeccakEnv, KeccakEnvironment},
    ArithOps, E,
};
use crate::mips::interpreter::{Lookup, LookupMode, LookupTable};
use ark_ff::Field;
use kimchi::circuits::polynomials::keccak::constants::{
    DIM, QUARTERS, SHIFTS, SHIFTS_LEN, STATE_LEN,
};

pub(crate) trait Lookups {
    type Column;
    type Variable: std::ops::Mul<Self::Variable, Output = Self::Variable>
        + std::ops::Add<Self::Variable, Output = Self::Variable>
        + std::ops::Sub<Self::Variable, Output = Self::Variable>
        + Clone;

    /// Adds a given Lookup to the environment
    fn add_lookup(&mut self, lookup: Lookup<Self::Variable>);

    /// Adds all lookups of Self
    fn lookups(&mut self, rw: LookupMode);

    /// Adds a lookup to the RangeCheck16 table
    fn lookup_rc16(&mut self, rw: LookupMode, flag: Self::Variable, value: Self::Variable);

    /// Adds a lookup to the Reset table
    fn lookup_reset(
        &mut self,
        rw: LookupMode,
        flag: Self::Variable,
        dense: Self::Variable,
        sparse: Self::Variable,
    );

    /// Adds a lookup to the Shift table
    fn lookup_sparse(&mut self, rw: LookupMode, flag: Self::Variable, value: Self::Variable);

    /// Adds a lookup to the Byte table
    fn lookup_byte(&mut self, rw: LookupMode, flag: Self::Variable, value: Self::Variable);

    /// Adds the lookups required for the sponge
    fn lookups_sponge(&mut self, rw: LookupMode);

    /// Adds the lookups required for Theta in the round
    fn lookups_round_theta(&mut self, rw: LookupMode);

    /// Adds the lookups required for PiRho in the round
    fn lookups_round_pirho(&mut self, rw: LookupMode);

    /// Adds the lookups required for Chi in the round
    fn lookups_round_chi(&mut self, rw: LookupMode);

    /// Adds the lookups required for Iota in the round
    fn lookups_round_iota(&mut self, rw: LookupMode);
}

impl<Fp: Field> Lookups for KeccakEnv<Fp> {
    type Column = KeccakColumn;
    type Variable = E<Fp>;

    fn add_lookup(&mut self, lookup: Lookup<Self::Variable>) {
        self.lookups.push(lookup);
    }

    fn lookups(&mut self, rw: LookupMode) {
        // TODO: preimage lookups (somewhere else)

        // SPONGE LOOKUPS
        self.lookups_sponge(rw);

        // ROUND LOOKUPS
        {
            // THETA LOOKUPS
            self.lookups_round_theta(rw);
            // PIRHO LOOKUPS
            self.lookups_round_pirho(rw);
            // CHI LOOKUPS
            self.lookups_round_chi(rw);
            // IOTA LOOKUPS
            self.lookups_round_iota(rw);
        }
    }

    fn lookup_rc16(&mut self, rw: LookupMode, flag: Self::Variable, value: Self::Variable) {
        self.add_lookup(Lookup {
            mode: rw,
            magnitude: flag,
            table_id: LookupTable::RangeCheck16Lookup,
            value: vec![value],
        });
    }

    fn lookup_reset(
        &mut self,
        rw: LookupMode,
        flag: Self::Variable,
        dense: Self::Variable,
        sparse: Self::Variable,
    ) {
        self.add_lookup(Lookup {
            mode: rw,
            magnitude: flag,
            table_id: LookupTable::ResetLookup,
            value: vec![dense, sparse],
        });
    }

    fn lookup_sparse(&mut self, rw: LookupMode, flag: Self::Variable, value: Self::Variable) {
        self.add_lookup(Lookup {
            mode: rw,
            magnitude: flag,
            table_id: LookupTable::SparseLookup,
            value: vec![value],
        });
    }

    fn lookup_byte(&mut self, rw: LookupMode, flag: Self::Variable, value: Self::Variable) {
        self.add_lookup(Lookup {
            mode: rw,
            magnitude: flag,
            table_id: LookupTable::ByteLookup,
            value: vec![value],
        });
    }

    fn lookups_sponge(&mut self, rw: LookupMode) {
        // PADDING LOOKUPS
        // Power of two corresponds to 2^pad_length
        // Pad suffixes correspond to 10*1 rule
        // Note: When FlagLength=0, TwoToPad=1, and all PadSuffix=0
        self.add_lookup(Lookup {
            mode: rw,
            magnitude: self.is_sponge(),
            table_id: LookupTable::PadLookup,
            value: vec![
                self.length(),
                self.two_to_pad(),
                self.pad_suffix(0),
                self.pad_suffix(1),
                self.pad_suffix(2),
                self.pad_suffix(3),
                self.pad_suffix(4),
            ],
        });
        // BYTES LOOKUPS
        for i in 0..200 {
            // Bytes are <2^8
            self.lookup_byte(rw, self.is_sponge(), self.sponge_bytes(i));
        }
        // SHIFTS LOOKUPS
        for i in 100..SHIFTS_LEN {
            // Shifts1, Shifts2, Shifts3 are in the Sparse table
            self.lookup_sparse(rw, self.is_sponge(), self.sponge_shifts(i));
        }
        for i in 0..STATE_LEN {
            // Shifts0 together with Bits composition by pairs are in the Reset table
            let dense = self.sponge_bytes(2 * i) + self.sponge_bytes(2 * i + 1) * Self::two_pow(8);
            self.lookup_reset(rw, self.is_sponge(), dense, self.sponge_shifts(i));
        }
    }

    fn lookups_round_theta(&mut self, rw: LookupMode) {
        for q in 0..QUARTERS {
            for x in 0..DIM {
                // Check that ThetaRemainderC < 2^64
                self.lookup_rc16(rw, self.is_round(), self.remainder_c(x, q));
                // Check ThetaExpandRotC is the expansion of ThetaDenseRotC
                self.lookup_reset(
                    rw,
                    self.is_round(),
                    self.dense_rot_c(x, q),
                    self.expand_rot_c(x, q),
                );
                // Check ThetaShiftC0 is the expansion of ThetaDenseC
                self.lookup_reset(
                    rw,
                    self.is_round(),
                    self.dense_c(x, q),
                    self.shifts_c(0, x, q),
                );
                // Check that the rest of ThetaShiftsC are in the Sparse table
                for i in 1..SHIFTS {
                    self.lookup_sparse(rw, self.is_round(), self.shifts_c(i, x, q));
                }
            }
        }
    }

    fn lookups_round_pirho(&mut self, rw: LookupMode) {
        for q in 0..QUARTERS {
            for x in 0..DIM {
                for y in 0..DIM {
                    // Check that PiRhoRemainderE < 2^64 and PiRhoQuotientE < 2^64
                    self.lookup_rc16(rw, self.is_round(), self.remainder_e(y, x, q));
                    self.lookup_rc16(rw, self.is_round(), self.quotient_e(y, x, q));
                    // Check PiRhoExpandRotE is the expansion of PiRhoDenseRotE
                    self.lookup_reset(
                        rw,
                        self.is_round(),
                        self.dense_rot_e(y, x, q),
                        self.expand_rot_e(y, x, q),
                    );
                    // Check PiRhoShift0E is the expansion of PiRhoDenseE
                    self.lookup_reset(
                        rw,
                        self.is_round(),
                        self.dense_e(y, x, q),
                        self.shifts_e(0, y, x, q),
                    );
                    // Check that the rest of PiRhoShiftsE are in the Sparse table
                    for i in 1..SHIFTS {
                        self.lookup_sparse(rw, self.is_round(), self.shifts_e(i, y, x, q));
                    }
                }
            }
        }
    }

    fn lookups_round_chi(&mut self, rw: LookupMode) {
        for i in 0..SHIFTS_LEN {
            // Check ChiShiftsB and ChiShiftsSum are in the Sparse table
            self.lookup_sparse(rw, self.is_round(), self.vec_shifts_b()[i].clone());
            self.lookup_sparse(rw, self.is_round(), self.vec_shifts_sum()[i].clone());
        }
    }

    fn lookups_round_iota(&mut self, rw: LookupMode) {
        for i in 0..QUARTERS {
            // Check round constants correspond with the current round
            self.add_lookup(Lookup {
                mode: rw,
                magnitude: self.is_round(),
                table_id: LookupTable::RoundConstantsLookup,
                value: vec![self.round(), self.round_constants()[i].clone()],
            });
        }
    }
}
