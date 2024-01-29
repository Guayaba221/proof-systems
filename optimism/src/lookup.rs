use ark_ff::{Field, One};

#[derive(Copy, Clone, Debug)]
pub enum Sign {
    Pos,
    Neg,
}

#[derive(Copy, Clone, Debug)]
pub enum LookupMode {
    Read,
    Write,
}

#[derive(Copy, Clone, Debug)]
pub enum LookupTable {
    MemoryLookup,
    RegisterLookup,
    // Single-column table of 2^16 entries with the sparse representation of all values
    SparseLookup,
    // Single-column table of all values in the range [0, 2^16)
    RangeCheck16Lookup,
    // Dual-column table of all values in the range [0, 2^16) and their sparse representation
    ResetLookup,
    // 24-row table with all possible values for round and their round constant in expanded form
    RoundConstantsLookup,
    // All [0..136] values of possible padding lengths, the value 2^len, and the 5 corresponding pad suffixes with the 10*1 rule
    PadLookup,
    // All values that can be stored in a byte (amortized table, better than model as RangeCheck16 (x and scaled x)
    ByteLookup,
    // Input/Output of Keccak steps
    KeccakStepLookup,
    // Syscalls communication channel
    SyscallLookup,
}

#[derive(Clone, Debug)]
pub struct Lookup<Fp> {
    pub mode: LookupMode,
    /// The number of times that this lookup value should be added to / subtracted from the lookup accumulator.    pub magnitude_contribution: Fp,
    pub magnitude: Fp,
    pub table_id: LookupTable,
    pub value: Vec<Fp>,
}

impl<Fp: std::fmt::Display + Field> std::fmt::Display for Lookup<Fp> {
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
    pub fn read_if(if_is_true: T, table_id: LookupTable, value: Vec<T>) -> Self {
        Self {
            mode: LookupMode::Read,
            magnitude: if_is_true,
            table_id,
            value,
        }
    }

    pub fn write_if(if_is_true: T, table_id: LookupTable, value: Vec<T>) -> Self {
        Self {
            mode: LookupMode::Write,
            magnitude: if_is_true,
            table_id,
            value,
        }
    }

    pub fn read_one(table_id: LookupTable, value: Vec<T>) -> Self {
        Self {
            mode: LookupMode::Read,
            magnitude: T::one(),
            table_id,
            value,
        }
    }

    pub fn write_one(table_id: LookupTable, value: Vec<T>) -> Self {
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
