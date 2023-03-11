use crate::snarky::{
    checked_runner::RunState, constraint_system::BasicSnarkyConstraint, cvar::FieldVar,
    traits::SnarkyType,
};
use ark_ff::PrimeField;

trait OutOfCircuitSnarkyType2<F> {
    type InCircuit;
}

impl<F> OutOfCircuitSnarkyType2<F> for bool
where
    F: PrimeField,
{
    type InCircuit = Boolean<F>;
}

/// A boolean variable.
#[derive(Debug, Clone)]
pub struct Boolean<F: PrimeField>(FieldVar<F>);

impl<F> SnarkyType<F> for Boolean<F>
where
    F: PrimeField,
{
    type Auxiliary = ();

    type OutOfCircuit = bool;

    const SIZE_IN_FIELD_ELEMENTS: usize = 1;

    fn to_cvars(&self) -> (Vec<FieldVar<F>>, Self::Auxiliary) {
        (vec![self.0.clone()], ())
    }

    fn from_cvars_unsafe(cvars: Vec<FieldVar<F>>, _aux: Self::Auxiliary) -> Self {
        assert_eq!(cvars.len(), Self::SIZE_IN_FIELD_ELEMENTS);
        Self(cvars[0].clone())
    }

    fn check(&self, cs: &mut RunState<F>) {
        // TODO: annotation?
        cs.assert_(
            Some("boolean check"),
            vec![BasicSnarkyConstraint::Boolean(self.0.clone())],
        );
    }

    fn constraint_system_auxiliary() -> Self::Auxiliary {}

    fn value_to_field_elements(value: &Self::OutOfCircuit) -> (Vec<F>, Self::Auxiliary) {
        if *value {
            (vec![F::one()], ())
        } else {
            (vec![F::zero()], ())
        }
    }

    fn value_of_field_elements(fields: Vec<F>, _aux: Self::Auxiliary) -> Self::OutOfCircuit {
        assert_eq!(fields.len(), 1);

        fields[0] != F::zero()
    }
}

impl<F> Boolean<F>
where
    F: PrimeField,
{
    pub fn true_() -> Self {
        Self(FieldVar::Constant(F::one()))
    }

    pub fn false_() -> Self {
        Self(FieldVar::Constant(F::zero()))
    }

    pub fn not(&self) -> Self {
        Self(Self::true_().0 - &self.0)
    }

    pub fn and(&self, other: &Self, cs: &mut RunState<F>, loc: &str) -> Self {
        Self(self.0.mul(&other.0, Some("bool.and"), loc, cs))
    }

    pub fn or(&self, other: &Self, loc: &str, cs: &mut RunState<F>) -> Self {
        let both_false = self.not().and(&other.not(), cs, loc);
        both_false.not()
    }

    pub fn any(xs: &[&Self], cs: &mut RunState<F>, loc: &str) -> Self {
        if xs.is_empty() {
            return Self::false_(); // TODO: shouldn't we panic instead?
        } else if xs.len() == 1 {
            return xs[0].clone();
        } else if xs.len() == 2 {
            return xs[0].or(xs[1], loc, cs); // TODO: is this better than below?
        }

        let zero = FieldVar::Constant(F::zero());

        let xs: Vec<_> = xs.iter().map(|x| &x.0).collect();
        let sum = FieldVar::sum(&xs);
        let all_zero = sum.equal(cs, loc, &zero);

        all_zero.not()
    }

    pub fn all(xs: &[Self], cs: &mut RunState<F>, loc: &str) -> Self {
        if xs.is_empty() {
            return Self::true_(); // TODO: shouldn't we panic instead?
        } else if xs.len() == 1 {
            return xs[0].clone();
        } else if xs.len() == 2 {
            return xs[0].and(&xs[1], cs, loc); // TODO: is this better than below?
        }

        let expected = FieldVar::Constant(F::from(xs.len() as u64));
        let xs: Vec<_> = xs.iter().map(|x| &x.0).collect();
        let sum = FieldVar::sum(&xs);

        sum.equal(cs, loc, &expected)
    }

    pub fn to_constant(&self) -> Option<bool> {
        match self.0 {
            FieldVar::Constant(x) => Some(x == F::one()),
            _ => None,
        }
    }

    pub fn xor(&self, other: &Self, state: &mut RunState<F>, loc: &str) -> Self {
        match (self.to_constant(), other.to_constant()) {
            (Some(true), _) => other.not(),
            (_, Some(true)) => self.not(),
            (Some(false), _) => other.clone(),
            (_, Some(false)) => self.clone(),
            (None, None) => {
                /*
                   (1 - 2 a) (1 - 2 b) = 1 - 2 c
                1 - 2 (a + b) + 4 a b = 1 - 2 c
                - 2 (a + b) + 4 a b = - 2 c
                (a + b) - 2 a b = c
                2 a b = a + b - c
                 */

                let self_clone = self.clone();
                let other_clone = other.clone();
                let res: Boolean<F> = state.compute_unsafe(loc, move |env| {
                    let _b1: bool = self_clone.read(env);
                    let _b2: bool = other_clone.read(env);

                    /*
                    let%bind res =
                      exists typ_unchecked
                        ~compute:
                          As_prover.(
                            map2 ~f:Bool.( <> ) (read typ_unchecked b1)
                              (read typ_unchecked b2))
                    in
                     */

                    todo!()
                });

                let x = &self.0 + &self.0;
                let y = &other.0;
                let z = &self.0 + &other.0 - &res.0;

                // TODO: annotation?
                state.assert_r1cs(Some("xor"), x, y.clone(), z);

                res
            }
        }
    }
}
