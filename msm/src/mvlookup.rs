//! Implement the protocol MVLookup <https://eprint.iacr.org/2022/1530.pdf>

use std::iter;

use ark_ff::{FftField, Field};
use kimchi::circuits::domains::EvaluationDomains;
use rand::{seq::SliceRandom, thread_rng, Rng};

// TODO: Add more built-in lookup tables
#[derive(Copy, Clone, Debug)]
pub enum LookupTable {
    RangeCheck16,
    /// Custom lookup table
    /// The index of the table is used as the ID, padded with the number of
    /// built-in tables.
    Custom(usize),
}

impl LookupTable {
    /// Assign a unique ID to the lookup tables.
    pub fn into_field<F: Field>(self) -> F {
        match self {
            LookupTable::RangeCheck16 => F::one(),
            LookupTable::Custom(id) => F::from(id as u64) + F::one(),
        }
    }
}

/// Generic structure to represent a (vector) lookup the table with ID
/// `table_id`.
/// The structure represents the individual fraction of the sum described in the
/// MVLookup protocol (for instance Eq. 8).
/// The table ID is added to the random linear combination formed with the
/// values. The combiner for the random linear combination is coined during the
/// proving phase by the prover.
#[derive(Debug, Clone)]
pub struct Lookup<F> {
    pub(crate) table_id: LookupTable,
    pub(crate) numerator: F,
    pub(crate) value: Vec<F>,
}

/// Represents a witness of one instance of the lookup argument
#[derive(Debug)]
pub struct LookupWitness<F> {
    /// A list of functions/looked-up values.
    /// The values are represented as:
    /// [ [f_{1}(1), ..., f_{1}(\omega^n)],
    ///   [f_{2}(1), ..., f_{2}(\omega^n)]
    ///     ...
    ///   [f_{m}(1), ..., f_{m}(\omega^n)]
    /// ]
    /// TODO: for efficiency, as we go through columns and after that row, we
    /// should reorganize this. While working on the interpreter, we might
    /// change this structure.
    /// TODO: for efficiency, we might want to have a single flat fixed-size
    /// array
    pub(crate) f: Vec<Vec<Lookup<F>>>,
    /// The table the lookup is performed on.
    pub(crate) t: Vec<Lookup<F>>,
    /// The multiplicity polynomial
    pub(crate) m: Vec<F>,
}

// This should be used only for testing purposes.
// It is not only in the test API because it is used at the moment in the
// main.rs. It should be moved to the test API when main.rs is replaced with
// real production code.
impl<F: FftField> LookupWitness<F> {
    /// Generate a random number of correct lookups in the table RangeCheck16
    pub fn random(domain: EvaluationDomains<F>) -> Self {
        let mut rng = thread_rng();
        // TODO: generate more random f
        let table_size: u64 = rng.gen_range(1..domain.d1.size);
        let table_id = rng.gen_range(1..1000);
        // Build a table of value we can look up
        let t: Vec<u64> = {
            // Generate distinct values to avoid to have to handle the
            // normalized multiplicity polynomial
            let mut n: Vec<u64> = (1..(table_size * 100)).collect();
            n.shuffle(&mut rng);
            n[0..table_size as usize].to_vec()
        };
        // permutation argument
        let f = {
            let mut f = t.clone();
            f.shuffle(&mut rng);
            f
        };
        let dummy_value = F::rand(&mut rng);
        let repeated_dummy_value: Vec<F> = {
            let r: Vec<F> = iter::repeat(dummy_value)
                .take((domain.d1.size - table_size) as usize)
                .collect();
            r
        };
        let t_evals = {
            let mut table = Vec::with_capacity(domain.d1.size as usize);
            table.extend(t.iter().map(|v| Lookup {
                table_id: LookupTable::Custom(table_id),
                numerator: -F::one(),
                value: vec![F::from(*v)],
            }));
            table.extend(
                repeated_dummy_value
                    .iter()
                    .map(|v| Lookup {
                        table_id: LookupTable::Custom(table_id),
                        numerator: -F::one(),
                        value: vec![*v],
                    })
                    .collect::<Vec<Lookup<F>>>(),
            );
            table
        };
        let f_evals: Vec<Lookup<F>> = {
            let mut table = Vec::with_capacity(domain.d1.size as usize);
            table.extend(f.iter().map(|v| Lookup {
                table_id: LookupTable::Custom(table_id),
                numerator: F::one(),
                value: vec![F::from(*v)],
            }));
            table.extend(
                repeated_dummy_value
                    .iter()
                    .map(|v| Lookup {
                        table_id: LookupTable::Custom(table_id),
                        numerator: F::one(),
                        value: vec![*v],
                    })
                    .collect::<Vec<Lookup<F>>>(),
            );
            table
        };
        let m = (0..domain.d1.size).map(|_| F::one()).collect();
        LookupWitness {
            f: vec![f_evals],
            t: t_evals,
            m,
        }
    }
}

/// Represents the proof of the lookup argument
/// It is parametrized by the type `T` which can be either:
/// - Polycomm<G: KimchiCurve> for the commitments
/// - F for the evaluations at zeta (resp. zeta omega).
/// FIXME: We should have a fixed number of m and h. Should we encode that in
/// the type?
#[derive(Debug, Clone)]
pub struct LookupProof<T> {
    // The multiplicity polynomials
    pub(crate) m: Vec<T>,
    // The polynomial keeping the sum of each row
    pub(crate) h: Vec<T>,
    // The "running-sum" over the rows, coined \phi
    pub(crate) sum: T,
}

/// Iterator implementation to abstract the content of the structure.
/// It can be used to iterate over the commitments (resp. the evaluations)
/// without requiring to have a look at the inner fields.
impl<'lt, G> IntoIterator for &'lt LookupProof<G> {
    type Item = &'lt G;
    type IntoIter = std::vec::IntoIter<&'lt G>;

    fn into_iter(self) -> Self::IntoIter {
        let n = self.h.len();
        let mut iter_contents = Vec::with_capacity(1 + n + 1);
        iter_contents.extend(&self.m);
        iter_contents.extend(&self.h);
        iter_contents.push(&self.sum);
        iter_contents.into_iter()
    }
}

pub mod prover {
    use ark_ff::Zero;
    use ark_poly::Evaluations;
    use ark_poly::{univariate::DensePolynomial, Radix2EvaluationDomain as D};
    use kimchi::circuits::domains::EvaluationDomains;
    use kimchi::curve::KimchiCurve;
    use mina_poseidon::FqSponge;
    use poly_commitment::commitment::{absorb_commitment, PolyComm};
    use poly_commitment::{OpenProof, SRS as _};

    use rayon::iter::IntoParallelIterator;
    use rayon::iter::ParallelIterator;

    use super::{Lookup, LookupWitness};

    pub struct Env<G: KimchiCurve> {
        pub lookup_counters_evals_d1: Vec<Evaluations<G::ScalarField, D<G::ScalarField>>>,
        pub lookup_counters_poly_d1: Vec<DensePolynomial<G::ScalarField>>,
        pub lookup_counters_comm_d1: Vec<PolyComm<G>>,

        pub lookup_terms_evals_d1: Vec<Evaluations<G::ScalarField, D<G::ScalarField>>>,
        pub lookup_terms_poly_d1: Vec<DensePolynomial<G::ScalarField>>,
        pub lookup_terms_comms_d1: Vec<PolyComm<G>>,

        pub lookup_aggregation_evals_d1: Evaluations<G::ScalarField, D<G::ScalarField>>,
        pub lookup_aggregation_poly_d1: DensePolynomial<G::ScalarField>,
        pub lookup_aggregation_comm_d1: PolyComm<G>,
    }

    impl<G: KimchiCurve> Env<G> {
        /// Create an environment for the prover to create a proof for the MVLookup protocol.
        /// The protocol does suppose that the individual lookup terms are
        /// committed as part of the columns.
        /// Therefore, the protocol only focus on commiting to the "grand
        /// product sum" and the "row-accumulated" values.
        pub fn create<
            OpeningProof: OpenProof<G>,
            Sponge: FqSponge<G::BaseField, G, G::ScalarField>,
        >(
            lookups: Vec<LookupWitness<G::ScalarField>>,
            domain: EvaluationDomains<G::ScalarField>,
            fq_sponge: &mut Sponge,
            srs: &OpeningProof::SRS,
        ) -> Self
        where
            OpeningProof::SRS: Sync,
        {
            // Polynomial m(X)
            let lookup_counters_evals_d1 = (&lookups)
                .into_par_iter()
                .map(|lookup| {
                    Evaluations::<G::ScalarField, D<G::ScalarField>>::from_vec_and_domain(
                        lookup.m.to_vec(),
                        domain.d1,
                    )
                })
                .collect::<Vec<Evaluations<G::ScalarField, D<G::ScalarField>>>>();

            let lookup_counters_poly_d1: Vec<DensePolynomial<G::ScalarField>> =
                (&lookup_counters_evals_d1)
                    .into_par_iter()
                    .map(|evals| evals.interpolate_by_ref())
                    .collect();

            let lookup_counters_comm_d1: Vec<PolyComm<G>> = (&lookup_counters_evals_d1)
                .into_par_iter()
                .map(|poly| srs.commit_evaluations_non_hiding(domain.d1, poly))
                .collect();

            lookup_counters_comm_d1
                .iter()
                .for_each(|comm| absorb_commitment(fq_sponge, comm));
            // -- end of m(X)

            // -- start computing the row sums h(X)
            // It will be used to compute the running sum in lookup_aggregation
            // Coin a combiner to perform vector lookup.
            // The row sums h are defined as
            // h(\omega^i) = \sum_{j = 0}^{m} (1/\beta + f_{j}(\omega^i)) - (1 / (\beta + t(\omega^i)))
            let vector_lookup_combiner = fq_sponge.challenge();

            // Coin an evaluation point for the rational functions
            let beta = fq_sponge.challenge();

            let lookup_terms_evals: Vec<Vec<G::ScalarField>> = lookups
                .into_iter()
                .map(|lookup| {
                    let LookupWitness { f, t, m: _ } = lookup;
                    let n = f.len();
                    // We compute first the denominators of all f_i and t. We gather them in
                    // a vector to perform a batch inversion.
                    // We include t in the denominator, therefore n + 1
                    let mut denominators = Vec::with_capacity((n + 1) * domain.d1.size as usize);
                    // Iterate over the rows
                    for j in 0..domain.d1.size {
                        // Iterate over individual columns (i.e. f_i and t)
                        for f_i in f.iter() {
                            let Lookup {
                                numerator: _,
                                table_id,
                                value,
                            } = &f_i[j as usize];
                            // x + r * y + r^2 * z + ... + r^n table_id
                            let combined_value: G::ScalarField =
                                value.iter().rev().fold(G::ScalarField::zero(), |x, y| {
                                    x * vector_lookup_combiner + y
                                }) * vector_lookup_combiner
                                    + table_id.into_field::<G::ScalarField>();

                            // beta + a_{i}
                            let lookup_denominator = beta + combined_value;
                            denominators.push(lookup_denominator);
                        }

                        // We process t now
                        let Lookup {
                            numerator: _,
                            table_id,
                            value,
                        } = &t[j as usize];
                        let combined_value: G::ScalarField =
                            value.iter().rev().fold(G::ScalarField::zero(), |x, y| {
                                x * vector_lookup_combiner + y
                            }) * vector_lookup_combiner
                                + table_id.into_field::<G::ScalarField>();

                        let lookup_denominator = beta + combined_value;
                        denominators.push(lookup_denominator);
                    }

                    ark_ff::fields::batch_inversion(&mut denominators);

                    // Evals is the sum on the individual columns for each row
                    let mut evals = Vec::with_capacity(domain.d1.size as usize);
                    let mut denominator_index = 0;

                    // We only need to add the numerator now
                    for j in 0..domain.d1.size {
                        let mut row_acc = G::ScalarField::zero();
                        for f_i in f.iter() {
                            let Lookup {
                                numerator,
                                table_id: _,
                                value: _,
                            } = &f_i[j as usize];
                            row_acc += *numerator * denominators[denominator_index];
                            denominator_index += 1;
                        }
                        // We process t now
                        let Lookup {
                            numerator,
                            table_id: _,
                            value: _,
                        } = &t[j as usize];
                        row_acc += *numerator * denominators[denominator_index];
                        denominator_index += 1;
                        evals.push(row_acc)
                    }
                    evals
                })
                .collect::<Vec<_>>();

            let lookup_terms_evals_d1: Vec<Evaluations<G::ScalarField, D<G::ScalarField>>> =
                lookup_terms_evals
                    .into_iter()
                    .map(|lte| {
                        Evaluations::<G::ScalarField, D<G::ScalarField>>::from_vec_and_domain(
                            lte, domain.d1,
                        )
                    })
                    .collect::<Vec<_>>();

            let lookup_terms_poly_d1: Vec<DensePolynomial<G::ScalarField>> =
                (&lookup_terms_evals_d1)
                    .into_par_iter()
                    .map(|lte| lte.interpolate_by_ref())
                    .collect::<Vec<_>>();

            let lookup_terms_comms_d1: Vec<PolyComm<G>> = (&lookup_terms_evals_d1)
                .into_par_iter()
                .map(|lte| srs.commit_evaluations_non_hiding(domain.d1, lte))
                .collect::<Vec<_>>();

            lookup_terms_comms_d1
                .iter()
                .for_each(|comm| absorb_commitment(fq_sponge, comm));
            // -- end computing the row sums h

            // -- start computing the running sum in lookup_aggregation
            // The running sum, \phi, is defined recursively over the subgroup as followed:
            // - phi(1) = 0
            // - phi(\omega^{j + 1}) = \phi(\omega^j) + \
            //                         \sum_{i = 1}^{n} (1 / \beta + f_i(\omega^{j + 1})) - \
            //                         (m(\omega^{j + 1}) / beta + t(\omega^{j + 1}))
            // - phi(\omega^n) = 0
            let lookup_aggregation_evals_d1 = {
                let mut evals = Vec::with_capacity(domain.d1.size as usize);
                let mut acc = G::ScalarField::zero();
                for i in 0..domain.d1.size as usize {
                    // phi(1) = 0
                    evals.push(acc);
                    for lte in lookup_terms_evals_d1.iter() {
                        acc += lte[i]
                    }
                }
                // Sanity check to verify that the accumulator ends up being zero.
                // FIXME: This should be removed from runtime, and a constraint
                // should be added. For now, the verifier accepts any proof.
                // This will be fixed when constraints are added.
                assert_eq!(acc, G::ScalarField::zero());
                Evaluations::<G::ScalarField, D<G::ScalarField>>::from_vec_and_domain(
                    evals, domain.d1,
                )
            };

            let lookup_aggregation_poly_d1 = lookup_aggregation_evals_d1.interpolate_by_ref();
            let lookup_aggregation_comm_d1 =
                srs.commit_evaluations_non_hiding(domain.d1, &lookup_aggregation_evals_d1);

            absorb_commitment(fq_sponge, &lookup_aggregation_comm_d1);
            Self {
                lookup_counters_evals_d1,
                lookup_counters_poly_d1,
                lookup_counters_comm_d1,

                lookup_terms_evals_d1,
                lookup_terms_poly_d1,
                lookup_terms_comms_d1,

                lookup_aggregation_evals_d1,
                lookup_aggregation_poly_d1,
                lookup_aggregation_comm_d1,
            }
        }
    }
}
