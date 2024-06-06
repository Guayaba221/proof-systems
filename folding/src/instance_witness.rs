//! This module defines a list of traits and structures that are used by the
//! folding scheme.
//! The folding library is built over generic traits like [Instance] and
//! [Witness] that defines the the NP relation.
//!
//! Here a brief description of the traits and structures:
//! - [Instance]: represents the instance (public input)
//! - [Witness]: represents the witness (private input)
//! - [ExtendedInstance]: represents the instance extended with extra columns
//! that are added by quadraticization
//! - [ExtendedWitness]: represents the witness extended with extra columns that
//! are added by quadraticization
//! - [RelaxedInstance]: represents a relaxed instance
//! - [RelaxedWitness]: represents a relaxed witness
//! - [RelaxableInstance]: a trait that allows to relax an instance. It is
//! implemented for [Instance] and [RelaxedInstance] so methods that require a
//! relaxed instance can also be called on a normal instance
//! - [RelaxableWitness]: same than [RelaxableInstance] but for witnesses.

// FIXME: for optimisation, as values are not necessarily Fp elements and are
// relatively small, we could get rid of the scalar field objects, and only use
// bigint where we only apply the modulus when needed.

use crate::{Alphas, Evals};
use ark_ff::Field;
use num_traits::One;
use poly_commitment::commitment::{CommitmentCurve, PolyComm};
use std::collections::BTreeMap;

pub trait Foldable<F: Field> {
    /// Combine two objects 'a' and 'b' into a new object using the challenge.
    // FIXME: rename in fold2
    fn combine(a: Self, b: Self, challenge: F) -> Self;
}

pub trait Instance<G: CommitmentCurve>: Sized + Foldable<G::ScalarField> {
    /// This method returns the scalars and commitments that must be absorbed by
    /// the sponge. It is not supposed to do any absorption itself, and the user
    /// is responsible for calling the sponge absorb methods with the elements
    /// returned by this method.
    /// When called on a RelaxedInstance, elements will be returned in the
    /// following order, for given instances L and R
    /// ```text
    /// scalar = L.to_absorb().0 | L.u | R.to_absorb().0 | R.u
    /// points_l = L.to_absorb().1 | L.extended | L.error // where extended is the commitments to the extra columns
    /// points_r = R.to_absorb().1 | R.extended | R.error // where extended is the commitments to the extra columns
    /// t_0 and t_1 first and second error terms
    /// points = points_l | points_r | t_0 | t_1
    /// ```
    /// A user implementing the IVC circuit should absorb the elements in the
    /// following order:
    /// ```text
    /// sponge.absorb_fr(scalar); // absorb the scalar elements
    /// sponge.absorb_g(points); // absorb the commitments
    /// ```
    /// This is the order used by the folding library in the method
    /// `fold_instance_witness_pair`.
    /// From there, a challenge can be coined using:
    /// ```text
    /// let challenge_r = sponge.challenge();
    /// ```
    fn to_absorb(&self) -> (Vec<G::ScalarField>, Vec<G>);

    /// This method takes an Instance and a commitment to zero and extends the instance,
    /// returning a relaxed instance which is composed by the extended instance, the scalar one,
    /// and the error commitment which is set to the commitment to zero.
    fn relax(self, zero_commit: PolyComm<G>) -> RelaxedInstance<G, Self> {
        let extended_instance = ExtendedInstance::extend(self);
        RelaxedInstance {
            extended_instance,
            u: G::ScalarField::one(),
            error_commitment: zero_commit,
        }
    }

    /// Returns the alphas values for the instance
    fn get_alphas(&self) -> &Alphas<G::ScalarField>;
}

pub trait Witness<G: CommitmentCurve>: Sized + Foldable<G::ScalarField> {
    /// This method takes a witness and a vector of evaluations to the zero polynomial,
    /// returning a relaxed witness which is composed by the extended witness and the error vector
    /// that is set to the zero polynomial.
    fn relax(self, zero_poly: &Evals<G::ScalarField>) -> RelaxedWitness<G, Self> {
        let extended_witness = ExtendedWitness::extend(self);
        RelaxedWitness {
            extended_witness,
            error_vec: zero_poly.clone(),
        }
    }
}

impl<G: CommitmentCurve, W: Witness<G>> ExtendedWitness<G, W> {
    /// This method returns an extended witness which is defined as the witness itself,
    /// followed by an empty BTreeMap.
    /// The map will be later filled by the quadraticization witness generator.
    fn extend(witness: W) -> ExtendedWitness<G, W> {
        let extended = BTreeMap::new();
        ExtendedWitness { witness, extended }
    }
}

impl<G: CommitmentCurve, I: Instance<G>> ExtendedInstance<G, I> {
    /// This method returns an extended instance which is defined as the instance itself,
    /// followed by an empty vector.
    fn extend(instance: I) -> ExtendedInstance<G, I> {
        ExtendedInstance {
            instance,
            extended: vec![],
        }
    }
}

/// A relaxed instance is an instance that has been relaxed by the folding scheme.
/// It contains the original instance, extended with the columns added by
/// quadriticization, the scalar `u` and a commitment to the
/// slack/error term.
/// See page 15 of [Nova](https://eprint.iacr.org/2021/370.pdf).
// FIXME: We should forbid cloning, for memory footprint.
#[derive(Clone)]
pub struct RelaxedInstance<G: CommitmentCurve, I: Instance<G>> {
    /// The original instance, extended with the columns added by
    /// quadriticization
    pub extended_instance: ExtendedInstance<G, I>,
    pub u: G::ScalarField,
    pub error_commitment: PolyComm<G>,
}

impl<G: CommitmentCurve, I: Instance<G>> RelaxedInstance<G, I> {
    /// Returns the elements to be absorbed by the sponge
    ///
    /// The scalar elements of the are appended with the scalar `u` and the
    /// commitments are appended by the commitment to the error term.
    pub fn to_absorb(&self) -> (Vec<G::ScalarField>, Vec<G>) {
        let mut elements = self.extended_instance.to_absorb();
        elements.0.push(self.u);
        assert_eq!(self.error_commitment.elems.len(), 1);
        elements.1.push(self.error_commitment.elems[0]);
        elements
    }

    /// Provides access to commitments to the extra columns added by
    /// quadraticization
    pub fn get_extended_column_commitment(&self, i: usize) -> Option<&PolyComm<G>> {
        self.extended_instance.extended.get(i)
    }
}

// -- Relaxed witnesses
pub struct RelaxedWitness<G: CommitmentCurve, W: Witness<G>> {
    pub extended_witness: ExtendedWitness<G, W>,
    pub error_vec: Evals<G::ScalarField>,
}

impl<G: CommitmentCurve, W: Witness<G>> RelaxedWitness<G, W> {
    /// Provides access to the extra columns added by quadraticization
    pub fn get_extended_column(&self, i: &usize) -> Option<&Evals<G::ScalarField>> {
        self.extended_witness.extended.get(i)
    }
}

// -- Extended witness
/// This structure represents a witness extended with extra columns that are
/// added by quadraticization
pub struct ExtendedWitness<G: CommitmentCurve, W: Witness<G>> {
    pub witness: W,
    /// Extra columns added by quadraticization to lower the degree of
    /// expressions to 2
    pub extended: BTreeMap<usize, Evals<G::ScalarField>>,
}

impl<G: CommitmentCurve, W: Witness<G>> Foldable<G::ScalarField> for ExtendedWitness<G, W> {
    fn combine(a: Self, b: Self, challenge: <G>::ScalarField) -> Self {
        let Self {
            witness: witness1,
            extended: ex1,
        } = a;
        let Self {
            witness: witness2,
            extended: ex2,
        } = b;
        let witness = W::combine(witness1, witness2, challenge);
        let extended = ex1
            .into_iter()
            .zip(ex2)
            .map(|(a, b)| {
                let (i, mut evals) = a;
                assert_eq!(i, b.0);
                evals
                    .evals
                    .iter_mut()
                    .zip(b.1.evals)
                    .for_each(|(a, b)| *a += b * challenge);
                (i, evals)
            })
            .collect();
        Self { witness, extended }
    }
}

impl<G: CommitmentCurve, W: Witness<G>> Witness<G> for ExtendedWitness<G, W> {}

impl<G: CommitmentCurve, W: Witness<G>> ExtendedWitness<G, W> {
    pub(crate) fn add_witness_evals(&mut self, i: usize, evals: Evals<G::ScalarField>) {
        self.extended.insert(i, evals);
    }

    /// Return true if the no extra columns are added by quadraticization
    ///
    /// Can be used to know if the extended witness columns are already
    /// computed, to avoid overriding them
    pub fn is_extended(&self) -> bool {
        !self.extended.is_empty()
    }
}

// -- Extended instance
/// An extended instance is an instance that has been extended with extra
/// columns by quadraticization.
/// The original instance is stored in the `instance` field.
/// The extra columns are stored in the `extended` field.
/// For instance, if the original instance has `n` columns, and the system is
/// described by a degree 3 polynomial, an additional column will be added, and
/// `extended` will contain `1` commitment.
// FIXME: We should forbid cloning, for memory footprint.
#[derive(Clone)]
pub struct ExtendedInstance<G: CommitmentCurve, I: Instance<G>> {
    /// The original instance.
    pub instance: I,
    /// Commitments to the extra columns added by quadraticization
    pub extended: Vec<PolyComm<G>>,
}

impl<G: CommitmentCurve, I: Instance<G>> Foldable<G::ScalarField> for ExtendedInstance<G, I> {
    fn combine(a: Self, b: Self, challenge: <G>::ScalarField) -> Self {
        let Self {
            instance: instance1,
            extended: ex1,
        } = a;
        let Self {
            instance: instance2,
            extended: ex2,
        } = b;
        let instance = I::combine(instance1, instance2, challenge);
        let extended = ex1
            .into_iter()
            .zip(ex2)
            .map(|(a, b)| &a + &b.scale(challenge))
            .collect();
        Self { instance, extended }
    }
}

impl<G: CommitmentCurve, I: Instance<G>> Instance<G> for ExtendedInstance<G, I> {
    /// Return the elements to be absorbed by the sponge
    ///
    /// The commitments to the additional columns created by quadriticization
    /// are appended to the existing commitments of the initial instance
    /// to be absorbed. The scalars are unchanged.
    fn to_absorb(&self) -> (Vec<G::ScalarField>, Vec<G>) {
        let mut elements = self.instance.to_absorb();
        let extended_commitments = self.extended.iter().map(|commit| {
            assert_eq!(commit.elems.len(), 1);
            commit.elems[0]
        });
        elements.1.extend(extended_commitments);
        elements
    }

    fn get_alphas(&self) -> &Alphas<G::ScalarField> {
        self.instance.get_alphas()
    }
}

// -- Relaxable instance
pub trait RelaxableInstance<G: CommitmentCurve, I: Instance<G>> {
    fn relax(self, zero_commitment: PolyComm<G>) -> RelaxedInstance<G, I>;
}

impl<G: CommitmentCurve, I: Instance<G>> RelaxableInstance<G, I> for I {
    fn relax(self, zero_commitment: PolyComm<G>) -> RelaxedInstance<G, I> {
        self.relax(zero_commitment)
    }
}

impl<G: CommitmentCurve, I: Instance<G>> RelaxableInstance<G, I> for RelaxedInstance<G, I> {
    fn relax(self, _zero_commitment: PolyComm<G>) -> RelaxedInstance<G, I> {
        self
    }
}

pub trait RelaxableWitness<G: CommitmentCurve, W: Witness<G>> {
    fn relax(self, zero_poly: &Evals<G::ScalarField>) -> RelaxedWitness<G, W>;
}

impl<G: CommitmentCurve, W: Witness<G>> RelaxableWitness<G, W> for W {
    fn relax(self, zero_poly: &Evals<G::ScalarField>) -> RelaxedWitness<G, W> {
        self.relax(zero_poly)
    }
}

impl<G: CommitmentCurve, W: Witness<G>> RelaxableWitness<G, W> for RelaxedWitness<G, W> {
    fn relax(self, _zero_poly: &Evals<G::ScalarField>) -> RelaxedWitness<G, W> {
        self
    }
}

pub trait RelaxablePair<G: CommitmentCurve, I: Instance<G>, W: Witness<G>> {
    fn relax(
        self,
        zero_poly: &Evals<G::ScalarField>,
        zero_commitment: PolyComm<G>,
    ) -> (RelaxedInstance<G, I>, RelaxedWitness<G, W>);
}

impl<G, I, W> RelaxablePair<G, I, W> for (I, W)
where
    G: CommitmentCurve,
    I: Instance<G> + RelaxableInstance<G, I>,
    W: Witness<G> + RelaxableWitness<G, W>,
{
    fn relax(
        self,
        zero_poly: &Evals<G::ScalarField>,
        zero_commitment: PolyComm<G>,
    ) -> (RelaxedInstance<G, I>, RelaxedWitness<G, W>) {
        let (instance, witness) = self;
        (
            RelaxableInstance::relax(instance, zero_commitment),
            RelaxableWitness::relax(witness, zero_poly),
        )
    }
}

impl<G, I, W> RelaxablePair<G, I, W> for (RelaxedInstance<G, I>, RelaxedWitness<G, W>)
where
    G: CommitmentCurve,
    I: Instance<G> + RelaxableInstance<G, I>,
    W: Witness<G> + RelaxableWitness<G, W>,
{
    fn relax(
        self,
        _zero_poly: &Evals<G::ScalarField>,
        _zero_commitment: PolyComm<G>,
    ) -> (RelaxedInstance<G, I>, RelaxedWitness<G, W>) {
        self
    }
}

impl<G: CommitmentCurve, I: Instance<G>> Foldable<G::ScalarField> for RelaxedInstance<G, I> {
    fn combine(a: Self, b: Self, challenge: <G>::ScalarField) -> Self {
        let challenge_cube = challenge * challenge * challenge;
        let RelaxedInstance {
            extended_instance: instance1,
            u: u1,
            error_commitment: e1,
        } = a;
        let RelaxedInstance {
            extended_instance: instance2,
            u: u2,
            error_commitment: e2,
        } = b;
        let extended_instance = <ExtendedInstance<G, I>>::combine(instance1, instance2, challenge);
        let u = u1 + u2 * challenge;
        let error_commitment = &e1 + &e2.scale(challenge_cube);
        RelaxedInstance {
            extended_instance,
            u,
            error_commitment,
        }
    }
}

impl<G: CommitmentCurve, I: Instance<G>> RelaxedInstance<G, I> {
    fn sub_errors(self, error_commitments: &[PolyComm<G>; 2], challenge: G::ScalarField) -> Self {
        let RelaxedInstance {
            extended_instance,
            u,
            error_commitment: error,
        } = self;
        let [e0, e1] = error_commitments;
        let error_commitment = &error - (&(&e0.scale(challenge) + &e1.scale(challenge.square())));
        RelaxedInstance {
            extended_instance,
            u,
            error_commitment,
        }
    }

    pub(super) fn combine_and_sub_error(
        a: Self,
        b: Self,
        challenge: <G>::ScalarField,
        error_commitments: &[PolyComm<G>; 2],
    ) -> Self {
        Self::combine(a, b, challenge).sub_errors(error_commitments, challenge)
    }
}

impl<G: CommitmentCurve, W: Witness<G>> Foldable<G::ScalarField> for RelaxedWitness<G, W> {
    fn combine(a: Self, b: Self, challenge: <G>::ScalarField) -> Self {
        let RelaxedWitness {
            extended_witness: a,
            error_vec: mut e1,
        } = a;
        let RelaxedWitness {
            extended_witness: b,
            error_vec: e2,
        } = b;
        let challenge_cube = (challenge * challenge) * challenge;
        let extended_witness = <ExtendedWitness<G, W>>::combine(a, b, challenge);
        for (a, b) in e1.evals.iter_mut().zip(e2.evals.into_iter()) {
            *a += b * challenge_cube;
        }
        let error_vec = e1;
        RelaxedWitness {
            extended_witness,
            error_vec,
        }
    }
}

impl<G: CommitmentCurve, W: Witness<G>> RelaxedWitness<G, W> {
    fn sub_error(mut self, errors: [Vec<G::ScalarField>; 2], challenge: G::ScalarField) -> Self {
        let [e0, e1] = errors;

        for (a, (e0, e1)) in self
            .error_vec
            .evals
            .iter_mut()
            .zip(e0.into_iter().zip(e1.into_iter()))
        {
            // FIXME: for optimisation, use inplace operators. Allocating can be
            // costly
            // should be the same as e0 * c + e1 * c^2
            *a -= ((e1 * challenge) + e0) * challenge;
        }
        self
    }

    pub(super) fn combine_and_sub_error(
        a: Self,
        b: Self,
        challenge: <G>::ScalarField,
        error: [Vec<G::ScalarField>; 2],
    ) -> Self {
        Self::combine(a, b, challenge).sub_error(error, challenge)
    }
}
