use ark_ff::FftField;
use ark_poly::{Evaluations, Radix2EvaluationDomain};

use crate::{logup, logup::LookupTableID, witness::Witness};
use kimchi::circuits::{
    domains::EvaluationDomains,
    expr::{Challenges, ColumnEnvironment as TColumnEnvironment, Constants, Domain},
};

/// The collection of polynomials (all in evaluation form) and constants
/// required to evaluate an expression as a polynomial.
///
/// All are evaluations.
pub struct ColumnEnvironment<
    'a,
    const N: usize,
    const N_REL: usize,
    const N_SEL: usize,
    F: FftField,
    ID: LookupTableID,
> {
    /// The witness column polynomials
    pub witness: &'a Witness<N, Evaluations<F, Radix2EvaluationDomain<F>>>,
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
    pub lookup: Option<logup::prover::QuotientPolynomialEnvironment<'a, F, ID>>,
}

impl<
        'a,
        const N: usize,
        const N_REL: usize,
        const N_SEL: usize,
        F: FftField,
        ID: LookupTableID,
    > TColumnEnvironment<'a, F> for ColumnEnvironment<'a, N, N_REL, N_SEL, F, ID>
{
    type Column = crate::columns::Column;

    fn get_column(
        &self,
        col: &Self::Column,
    ) -> Option<&'a Evaluations<F, Radix2EvaluationDomain<F>>> {
        // TODO: when non-literal constant generics are available, substitute N with N_REG + N_SEL
        assert!(N == N_REL + N_SEL);
        assert!(N == self.witness.len());
        match *col {
            // Handling the "relation columns" at the beginning of the witness columns
            Self::Column::Relation(i) => {
                if i < N_REL {
                    let res = &self.witness[i];
                    Some(res)
                } else {
                    // TODO: add a test for this
                    panic!("Requested column with index {:?} but the given witness is meant for {:?} relation columns", i, N_REL)
                }
            }
            // Handling the "dynamic selector columns" at the end of the witness columns
            Self::Column::DynamicSelector(i) => {
                if i < N_SEL {
                    let res = &self.witness[N_REL + i];
                    Some(res)
                } else {
                    panic!("Requested selector with index {:?} but the given witness is meant for {:?} dynamic selector columns", i, N_SEL)
                }
            }
            Self::Column::LookupPartialSum((table_id, i)) => {
                if let Some(ref lookup) = self.lookup {
                    let table_id = ID::from_u32(table_id);
                    Some(&lookup.lookup_terms_evals_d8[&table_id][i])
                } else {
                    panic!("No lookup provided")
                }
            }
            Self::Column::LookupAggregation => {
                if let Some(ref lookup) = self.lookup {
                    Some(lookup.lookup_aggregation_evals_d8)
                } else {
                    panic!("No lookup provided")
                }
            }
            Self::Column::LookupMultiplicity(table_id) => {
                if let Some(ref lookup) = self.lookup {
                    Some(&lookup.lookup_counters_evals_d8[&ID::from_u32(table_id)])
                } else {
                    panic!("No lookup provided")
                }
            }
            Self::Column::LookupFixedTable(table_id) => {
                if let Some(ref lookup) = self.lookup {
                    Some(&lookup.fixed_tables_evals_d8[&ID::from_u32(table_id)])
                } else {
                    panic!("No lookup provided")
                }
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

    fn column_domain(&self, col: &Self::Column) -> Domain {
        match *col {
            Self::Column::Relation(i) | Self::Column::DynamicSelector(i) => {
                let domain_size = if *col == Self::Column::Relation(i) {
                    // Relation
                    self.witness[i].domain().size
                } else {
                    // DynamicSelector
                    self.witness[N_REL + i].domain().size
                };
                if self.domain.d1.size == domain_size {
                    Domain::D1
                } else if self.domain.d2.size == domain_size {
                    Domain::D2
                } else if self.domain.d4.size == domain_size {
                    Domain::D4
                } else if self.domain.d8.size == domain_size {
                    Domain::D8
                } else {
                    panic!("Domain not supported. We do support the following multiple of the domain registered in the environment: 1, 2, 4, 8")
                }
            }
            Self::Column::LookupAggregation
            | Self::Column::LookupFixedTable(_)
            | Self::Column::LookupMultiplicity(_)
            | Self::Column::LookupPartialSum(_) => {
                // When there is a lookup, we do suppose the domain is always D8
                // and we have at leat 6 lookups per row.
                Domain::D8
            }
        }
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
