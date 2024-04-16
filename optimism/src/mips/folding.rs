use crate::{
    folding::{Challenge, Curve, FoldingEnvironment, FoldingInstance, FoldingWitness, Fp},
    mips::column::{ColumnAlias as MIPSColumn, MIPS_COLUMNS},
    DOMAIN_SIZE,
};
use ark_poly::{Evaluations, Radix2EvaluationDomain};
use kimchi::folding::{expressions::FoldingColumnTrait, BaseSponge, FoldingConfig};
use std::ops::Index;

pub(crate) type MIPSFoldingWitness = FoldingWitness<MIPS_COLUMNS>;
pub(crate) type MIPSFoldingInstance = FoldingInstance<MIPS_COLUMNS>;
pub(crate) type MIPSFoldingEnvironment = FoldingEnvironment<MIPS_COLUMNS, MIPSStructure>;

impl Index<MIPSColumn> for MIPSFoldingWitness {
    type Output = Evaluations<Fp, Radix2EvaluationDomain<Fp>>;

    fn index(&self, index: MIPSColumn) -> &Self::Output {
        &self.witness[index]
    }
}

// TODO: will contain information about the circuit structure
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MIPSStructure;

// TODO
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MIPSConfig;

impl FoldingColumnTrait for MIPSColumn {
    fn is_witness(&self) -> bool {
        // All MIPS columns are witness columns
        true
    }
}

impl FoldingConfig for MIPSConfig {
    type Column = MIPSColumn;
    type Challenge = Challenge;
    type Curve = Curve;
    type Srs = poly_commitment::srs::SRS<Curve>;
    type Sponge = BaseSponge;
    type Instance = MIPSFoldingInstance;
    type Witness = MIPSFoldingWitness;
    type Structure = MIPSStructure;
    type Env = MIPSFoldingEnvironment;

    fn rows() -> usize {
        DOMAIN_SIZE
    }
}
