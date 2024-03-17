use ark_ff::FftField;
use ark_poly::{Evaluations, Radix2EvaluationDomain};

use crate::mvlookup;
use crate::witness::Witness;
use kimchi::circuits::domains::EvaluationDomains;
use kimchi::circuits::expr::{
    Challenges, ColumnEnvironment as TColumnEnvironment, Constants, Domain,
};

/// The collection of polynomials (all in evaluation form) and constants
/// required to evaluate an expression as a polynomial.
///
/// All are evaluations.
pub struct ColumnEnvironment<'a, const N: usize, F: FftField> {
    /// The witness column polynomials
    pub witness: &'a Witness<N, Evaluations<F, Radix2EvaluationDomain<F>>>,
    /// The coefficient column polynomials
    pub coefficients: &'a Vec<Evaluations<F, Radix2EvaluationDomain<F>>>,
    /// The value `prod_{j != 1} (1 - omega^j)`, used for efficiently
    /// computing the evaluations of the unnormalized Lagrange basis polynomials.
    pub l0_1: F,
    /// Constant values required
    pub constants: Constants<F>,
    /// Challenges from the IOP.
    pub challenges: Challenges<F>,
    /// The domains used in the PLONK argument.
    pub domain: EvaluationDomains<F>,
    /// Lookup specific polynomials
    // TODO: rename in additive lookup or "logup"
    // We do not use multi-variate lookups, only the additive part
    pub lookup: Option<mvlookup::prover::QuotientPolynomialEnvironment<'a, F>>,
}

impl<'a, const N: usize, F: FftField> TColumnEnvironment<'a, F> for ColumnEnvironment<'a, N, F> {
    type Column = crate::columns::Column;

    fn get_column(
        &self,
        col: &Self::Column,
    ) -> Option<&'a Evaluations<F, Radix2EvaluationDomain<F>>> {
        let witness_length: usize = self.witness.len();
        let coefficients_length: usize = self.coefficients.len();
        match *col {
            // Handling the "relation columns"
            Self::Column::X(i) => {
                if i < witness_length {
                    let res = &self.witness[i];
                    Some(res)
                } else if i < witness_length + coefficients_length {
                    // FIXME and add a test
                    Some(&self.coefficients[i])
                } else {
                    // TODO: add a test for this
                    panic!("Requested column with index {:?} but the given witness is meant for {:?} columns", i, witness_length)
                }
            }
            Self::Column::LookupPartialSum(i) => {
                if let Some(ref lookup) = self.lookup {
                    Some(&lookup.lookup_terms_evals_d1[i])
                } else {
                    panic!("No lookup provided")
                }
            }
            Self::Column::LookupAggregation => {
                if let Some(ref lookup) = self.lookup {
                    Some(lookup.lookup_aggregation_evals_d1)
                } else {
                    panic!("No lookup provided")
                }
            }
            Self::Column::LookupMultiplicity(i) => {
                if let Some(ref lookup) = self.lookup {
                    Some(&lookup.lookup_counters_evals_d1[i as usize])
                } else {
                    panic!("No lookup provided")
                }
            }
            Self::Column::LookupFixedTable(_) => {
                panic!("Logup is not yet implemented.")
            }
        }
    }

    fn get_domain(&self, d: Domain) -> Radix2EvaluationDomain<F> {
        match d {
            Domain::D1 => self.domain.d1,
            Domain::D2 => self.domain.d2,
            Domain::D4 => self.domain.d4,
            Domain::D8 => self.domain.d8,
        }
    }

    fn column_domain(&self, _col: &Self::Column) -> Domain {
        // TODO FIXME check this is a tricky variable it should match the evalution in column
        // this must be bigger or equal than degree chosen in runtime inside evaluations() for
        // evaluating an expression = degree of expression that is evaluated
        // And also ... in some cases... bigger than the witness column size? Equal?
        Domain::D4
    }

    fn get_constants(&self) -> &Constants<F> {
        &self.constants
    }

    fn get_challenges(&self) -> &Challenges<F> {
        &self.challenges
    }

    fn vanishes_on_zero_knowledge_and_previous_rows(
        &self,
    ) -> &'a Evaluations<F, Radix2EvaluationDomain<F>> {
        panic!("Not supposed to be used in MSM")
    }

    fn l0_1(&self) -> F {
        self.l0_1
    }
}
