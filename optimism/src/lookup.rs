use ark_ff::{Field, One};
use kimchi::{
    circuits::polynomials::keccak::{
        constants::{RATE_IN_BYTES, ROUNDS},
        Keccak, RC,
    },
    o1_utils::Two,
};

use crate::keccak::witness::pad_blocks;

pub(crate) const TWO_TO_16_UPPERBOUND: u32 = 1 << 16;

#[derive(Copy, Clone, Debug)]
pub enum LookupMode {
    Read,
    Write,
}

#[derive(Copy, Clone, Debug)]
pub enum LookupTableIDs {
    // RAM Tables
    MemoryLookup = 0,
    RegisterLookup = 1,
    /// Syscalls communication channel
    SyscallLookup = 2,
    /// Input/Output of Keccak steps
    KeccakStepLookup = 3,

    // Read Tables
    /// Single-column table of all values in the range [0, 2^16)
    RangeCheck16Lookup = 4,
    /// Single-column table of 2^16 entries with the sparse representation of all values
    SparseLookup = 5,
    /// Dual-column table of all values in the range [0, 2^16) and their sparse representation
    ResetLookup = 6,
    /// 24-row table with all possible values for round and their round constant in expanded form (in big endian)
    RoundConstantsLookup = 7,
    /// All [1..136] values of possible padding lengths, the value 2^len, and the 5 corresponding pad suffixes with the 10*1 rule
    PadLookup = 8,
    /// All values that can be stored in a byte (amortized table, better than model as RangeCheck16 (x and scaled x)
    ByteLookup = 9,
}

#[derive(Clone, Debug)]
pub struct Lookup<T> {
    pub mode: LookupMode,
    /// The number of times that this lookup value should be added to / subtracted from the lookup accumulator.
    pub magnitude: T,
    pub table_id: LookupTableIDs,
    pub value: Vec<T>,
}

impl<F: std::fmt::Display + Field> std::fmt::Display for Lookup<F> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let numerator = match self.mode {
            LookupMode::Read => self.magnitude,
            LookupMode::Write => -self.magnitude,
        };
        write!(
            formatter,
            "numerator: {}\ntable_id: {:?}\nvalue:\n[\n",
            numerator, self.table_id
        )?;
        for value in self.value.iter() {
            writeln!(formatter, "\t{}", value)?;
        }
        write!(formatter, "]")?;
        Ok(())
    }
}

impl<T: One> Lookup<T> {
    /// Reads one value when `if_is_true` is 1.
    pub fn read_if(if_is_true: T, table_id: LookupTableIDs, value: Vec<T>) -> Self {
        Self {
            mode: LookupMode::Read,
            magnitude: if_is_true,
            table_id,
            value,
        }
    }

    /// Writes one value when `if_is_true` is 1.
    pub fn write_if(if_is_true: T, table_id: LookupTableIDs, value: Vec<T>) -> Self {
        Self {
            mode: LookupMode::Write,
            magnitude: if_is_true,
            table_id,
            value,
        }
    }

    /// Reads one value from a table.
    pub fn read_one(table_id: LookupTableIDs, value: Vec<T>) -> Self {
        Self {
            mode: LookupMode::Read,
            magnitude: T::one(),
            table_id,
            value,
        }
    }

    /// Writes one value to a table.
    pub fn write_one(table_id: LookupTableIDs, value: Vec<T>) -> Self {
        Self {
            mode: LookupMode::Write,
            magnitude: T::one(),
            table_id,
            value,
        }
    }
}

/// This trait adds basic methods to deal with lookups inside an environment
pub trait Lookups {
    type Column;
    type Variable: std::ops::Mul<Self::Variable, Output = Self::Variable>
        + std::ops::Add<Self::Variable, Output = Self::Variable>
        + std::ops::Sub<Self::Variable, Output = Self::Variable>
        + Clone;

    /// Adds a given Lookup to the environment
    fn add_lookup(&mut self, lookup: Lookup<Self::Variable>);

    /// Adds all lookups of Self to the environment
    fn lookups(&mut self);
}

/// A table of values that can be used for a lookup, along with the ID for the table.
#[derive(Debug, Clone)]
pub struct LookupTable<F> {
    /// Table ID corresponding to this table
    #[allow(dead_code)]
    table_id: LookupTableIDs,
    /// Vector of values inside each entry of the table
    #[allow(dead_code)]
    entries: Vec<Vec<F>>,
}

impl<F: Field> LookupTable<F> {
    #[allow(dead_code)]
    fn table_terms(&self, mixer: F) -> Vec<F> {
        self.entries
            .iter()
            .map(|entry| {
                entry
                    .iter()
                    .fold(F::from(self.table_id as u32), |acc, value| {
                        acc + *value * mixer
                    })
            })
            .collect()
    }

    #[allow(dead_code)]
    fn table_range_check_16() -> Self {
        Self {
            table_id: LookupTableIDs::RangeCheck16Lookup,
            entries: (0..TWO_TO_16_UPPERBOUND)
                .map(|i| vec![F::from(i)])
                .collect(),
        }
    }

    #[allow(dead_code)]
    fn table_sparse() -> Self {
        Self {
            table_id: LookupTableIDs::SparseLookup,
            entries: (0..TWO_TO_16_UPPERBOUND)
                .map(|i| {
                    vec![F::from(
                        u64::from_str_radix(&format!("{:b}", i), 16).unwrap(),
                    )]
                })
                .collect(),
        }
    }

    #[allow(dead_code)]
    fn table_reset() -> Self {
        Self {
            table_id: LookupTableIDs::ResetLookup,
            entries: (0..TWO_TO_16_UPPERBOUND)
                .map(|i| {
                    vec![
                        F::from(i),
                        F::from(u64::from_str_radix(&format!("{:b}", i), 16).unwrap()),
                    ]
                })
                .collect(),
        }
    }

    #[allow(dead_code)]
    fn table_round_constants() -> Self {
        Self {
            table_id: LookupTableIDs::RoundConstantsLookup,
            entries: (0..=ROUNDS)
                .map(|i| {
                    vec![
                        F::from(i as u32),
                        F::from(Keccak::sparse(RC[i])[3]),
                        F::from(Keccak::sparse(RC[i])[2]),
                        F::from(Keccak::sparse(RC[i])[1]),
                        F::from(Keccak::sparse(RC[i])[0]),
                    ]
                })
                .collect(),
        }
    }

    #[allow(dead_code)]
    fn table_pad() -> Self {
        Self {
            table_id: LookupTableIDs::PadLookup,
            entries: (1..=RATE_IN_BYTES)
                .map(|i| {
                    let suffix = pad_blocks(i);
                    vec![
                        F::from(i as u64),
                        F::two_pow(i as u64),
                        suffix[0],
                        suffix[1],
                        suffix[2],
                        suffix[3],
                        suffix[4],
                    ]
                })
                .collect(),
        }
    }

    #[allow(dead_code)]
    fn table_byte() -> Self {
        Self {
            table_id: LookupTableIDs::ByteLookup,
            entries: (0..(1 << 8) as u32).map(|i| vec![F::from(i)]).collect(),
        }
    }
}
