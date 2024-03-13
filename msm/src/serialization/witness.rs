use ark_ff::PrimeField;
use num_bigint::BigUint;
use o1_utils::FieldHelpers;

use crate::columns::Column;
use crate::serialization::interpreter::InterpreterEnv;
use crate::serialization::{Lookup, LookupTable};
use crate::N_LIMBS;
use kimchi::circuits::domains::EvaluationDomains;
use std::iter;

use super::N_INTERMEDIATE_LIMBS;

/// Environment for the serializer interpreter
pub struct Env<Fp> {
    pub current_kimchi_limbs: [Fp; 3],
    /// The LIMB_NUM limbs that is used to encode a field element for the MSM
    pub msm_limbs: [Fp; N_LIMBS],
    /// Used for the decomposition in base 4 of the last limb of the foreign
    /// field Kimchi gate
    pub intermediate_limbs: [Fp; N_INTERMEDIATE_LIMBS],

    /// Keep track of the RangeCheck4 lookup multiplicities
    // Boxing to avoid stack overflow
    pub lookup_multiplicities_rangecheck4: Box<[Fp; 1 << 4]>,

    /// Keep track of the RangeCheck4 table multiplicities.
    /// The value `0` is used as a (repeated) dummy value.
    // Boxing to avoid stack overflow
    pub lookup_t_multiplicities_rangecheck4: Box<[Fp; 1 << 4]>,

    /// Keep track of the RangeCheck15 lookup multiplicities
    /// No t multiplicities as we do suppose we have a domain of
    /// size `1 << 15`
    // Boxing to avoid stack overflow
    pub lookup_multiplicities_rangecheck15: Box<[Fp; 1 << 15]>,

    /// Keep track of the rangecheck 4 lookups for each row.
    pub rangecheck4_lookups: Vec<Lookup<Fp>>,

    /// Keep track of the rangecheck 15 lookups for each row.
    pub rangecheck15_lookups: Vec<Lookup<Fp>>,
}

impl<Fp: PrimeField> InterpreterEnv<Fp> for Env<Fp> {
    type Position = Column;

    // Requiring an Fp element as we would need to compute values up to 180 bits
    // in the 15 bits decomposition.
    type Variable = Fp;

    fn add_constraint(&mut self, cst: Self::Variable) {
        assert_eq!(cst, Fp::zero());
    }

    fn constant(value: Fp) -> Self::Variable {
        value
    }

    fn get_column_for_kimchi_limb(j: usize) -> Self::Position {
        assert!(j < 3);
        Column::X(j)
    }

    fn get_column_for_intermediate_limb(j: usize) -> Self::Position {
        assert!(j < N_INTERMEDIATE_LIMBS);
        Column::X(3 + N_LIMBS + j)
    }

    fn range_check15(&mut self, value: &Self::Variable) {
        let value_biguint = value.to_biguint();
        assert!(value_biguint < BigUint::from(2u128.pow(15)));
        // Adding multiplicities
        let value_usize: usize = value_biguint.clone().try_into().unwrap();
        self.lookup_multiplicities_rangecheck15[value_usize] += Fp::one();
        self.rangecheck15_lookups.push(Lookup {
            table_id: LookupTable::RangeCheck15,
            numerator: Fp::one(),
            value: vec![*value],
        })
    }

    fn range_check4(&mut self, value: &Self::Variable) {
        let value_biguint = value.to_biguint();
        assert!(value_biguint < BigUint::from(2u128.pow(4)));
        // Adding multiplicities
        let value_usize: usize = value_biguint.clone().try_into().unwrap();
        self.lookup_multiplicities_rangecheck4[value_usize] += Fp::one();
        self.rangecheck4_lookups.push(Lookup {
            table_id: LookupTable::RangeCheck4,
            numerator: Fp::one(),
            value: vec![*value],
        })
    }

    fn copy(&mut self, x: &Self::Variable, position: Self::Position) -> Self::Variable {
        self.write_column(position, *x);
        *x
    }

    fn get_column_for_msm_limb(j: usize) -> Self::Position {
        assert!(j < N_LIMBS);
        Column::X(3 + j)
    }

    /// Returns the bits between [highest_bit, lowest_bit] of the variable `x`,
    /// and copy the result in the column `position`.
    /// The value `x` is expected to be encoded in big-endian
    fn bitmask_be(
        &mut self,
        x: &Self::Variable,
        highest_bit: u32,
        lowest_bit: u32,
        position: Self::Position,
    ) -> Self::Variable {
        // FIXME: we can assume bitmask_be will be called only on value with
        // maximum 128 bits. We use bitmask_be only for the limbs
        let x_bytes_u8 = &x.to_bytes()[0..16];
        let x_u128 = u128::from_le_bytes(x_bytes_u8.try_into().unwrap());
        let res = (x_u128 >> lowest_bit) & ((1 << (highest_bit - lowest_bit)) - 1);
        let res_fp: Fp = res.into();
        self.write_column(position, res_fp);
        res_fp
    }
}

impl<Fp: PrimeField> Env<Fp> {
    pub fn write_column(&mut self, position: Column, value: Fp) {
        match position {
            Column::X(i) => {
                if i < 3 {
                    self.current_kimchi_limbs[i] = value
                } else if i < 3 + N_LIMBS {
                    self.msm_limbs[i - 3] = value;
                } else if i < 3 + N_LIMBS + N_INTERMEDIATE_LIMBS {
                    self.intermediate_limbs[i - 3 - N_LIMBS] = value;
                } else {
                    panic!("Invalid column index")
                }
            }
            Column::LookupPartialSum(_) => {
                panic!(
                    "This is a lookup related column. The environment is
                supposed to write only in witness columns"
                );
            }
            Column::LookupMultiplicity(_) => {
                panic!(
                    "This is a lookup related column. The environment is
                supposed to write only in witness columns"
                );
            }
            Column::LookupAggregation => {
                panic!(
                    "This is a lookup related column. The environment is
                supposed to write only in witness columns"
                );
            }
            Column::LookupFixedTable(_) => {
                panic!(
                    "This is a lookup related column. The environment is
                supposed to write only in witness columns"
                );
            }
        }
    }

    pub fn add_rangecheck4_table_value(&mut self, i: usize) {
        if i < (1 << 4) {
            self.lookup_t_multiplicities_rangecheck4[i] += Fp::one();
        } else {
            self.lookup_t_multiplicities_rangecheck4[0] += Fp::one();
        }
    }

    pub fn reset(&mut self) {
        self.rangecheck15_lookups = vec![];
        self.rangecheck4_lookups = vec![];
    }

    /// Return the normalized multiplicity vector of RangeCheck4 in case the
    /// table is not injective. Note that it is the case for `RangeCheck4`.
    pub fn get_rangecheck4_normalized_multipliticies(
        &self,
        domain: EvaluationDomains<Fp>,
    ) -> Vec<Fp> {
        let mut m = vec![Fp::zero(); 1 << 4];
        self.lookup_multiplicities_rangecheck4
            .into_iter()
            .zip(self.lookup_t_multiplicities_rangecheck4.iter())
            .enumerate()
            .for_each(|(i, (m_f, m_t))| m[i] = m_f / m_t);
        let repeated_dummy_value: Vec<Fp> = iter::repeat(m[0])
            .take((domain.d1.size - (1 << 4)) as usize)
            .collect();
        m.extend(repeated_dummy_value);
        m
    }
    /// Return the normalized multiplicity vector of RangeCheck4 in case the
    /// table is not injective. Note that it is not the case for `RangeCheck15` as
    /// we assume the domain size is `1 << 15`.
    pub fn get_rangecheck15_normalized_multipliticies(
        &self,
        domain: EvaluationDomains<Fp>,
    ) -> Vec<Fp> {
        assert_eq!(domain.d1.size, 1 << 15);
        self.lookup_multiplicities_rangecheck15.to_vec()
    }
}

impl<Fp: PrimeField> Env<Fp> {
    pub fn create() -> Self {
        Self {
            current_kimchi_limbs: [Fp::zero(); 3],
            msm_limbs: [Fp::zero(); N_LIMBS],
            intermediate_limbs: [Fp::zero(); N_INTERMEDIATE_LIMBS],

            lookup_multiplicities_rangecheck4: Box::new([Fp::zero(); 1 << 4]),
            lookup_t_multiplicities_rangecheck4: Box::new([Fp::zero(); 1 << 4]),

            lookup_multiplicities_rangecheck15: Box::new([Fp::zero(); 1 << 15]),
            rangecheck4_lookups: vec![],
            rangecheck15_lookups: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::serialization::N_INTERMEDIATE_LIMBS;
    use crate::{LIMB_BITSIZE, N_LIMBS};

    use super::Env;
    use crate::serialization::interpreter::deserialize_field_element;
    use ark_ff::BigInteger;
    use ark_ff::FpParameters as _;
    use ark_ff::PrimeField;
    use ark_ff::{One, UniformRand, Zero};
    use mina_curves::pasta::Fp;
    use num_bigint::BigUint;
    use o1_utils::{tests::make_test_rng, FieldHelpers};
    use rand::Rng;

    fn test_decomposition_generic(x: Fp) {
        let bits = x.to_bits();
        let limb0: u128 = {
            let limb0_le_bits: &[bool] = &bits.clone().into_iter().take(88).collect::<Vec<bool>>();
            let limb0 = Fp::from_bits(limb0_le_bits).unwrap();
            limb0.to_biguint().try_into().unwrap()
        };
        let limb1: u128 = {
            let limb0_le_bits: &[bool] = &bits
                .clone()
                .into_iter()
                .skip(88)
                .take(88)
                .collect::<Vec<bool>>();
            let limb0 = Fp::from_bits(limb0_le_bits).unwrap();
            limb0.to_biguint().try_into().unwrap()
        };
        let limb2: u128 = {
            let limb0_le_bits: &[bool] = &bits
                .clone()
                .into_iter()
                .skip(2 * 88)
                .take(79)
                .collect::<Vec<bool>>();
            let limb0 = Fp::from_bits(limb0_le_bits).unwrap();
            limb0.to_biguint().try_into().unwrap()
        };
        let mut dummy_env = Env::<Fp>::create();
        deserialize_field_element(&mut dummy_env, [limb0, limb1, limb2]);

        // Check limb are copied into the environment
        assert_eq!(Fp::from(limb0), dummy_env.current_kimchi_limbs[0]);
        assert_eq!(Fp::from(limb1), dummy_env.current_kimchi_limbs[1]);
        assert_eq!(Fp::from(limb2), dummy_env.current_kimchi_limbs[2]);

        // Check intermediate limbs
        {
            let bits = Fp::from(limb2).to_bits();
            for j in 0..N_INTERMEDIATE_LIMBS {
                let le_bits: &[bool] = &bits
                    .clone()
                    .into_iter()
                    .skip(j * 4)
                    .take(4)
                    .collect::<Vec<bool>>();
                let t = Fp::from_bits(le_bits).unwrap();
                assert_eq!(
                    t,
                    dummy_env.intermediate_limbs[j],
                    "{}",
                    format_args!(
                        "Intermediate limb {j}. Exp value is {:?}, computed is {:?}",
                        t.to_biguint(),
                        dummy_env.intermediate_limbs[j].to_biguint()
                    )
                )
            }
        }

        // Checking msm limbs
        for i in 0..N_LIMBS {
            let le_bits: &[bool] = &bits
                .clone()
                .into_iter()
                .skip(i * LIMB_BITSIZE)
                .take(LIMB_BITSIZE)
                .collect::<Vec<bool>>();
            let t = Fp::from_bits(le_bits).unwrap();
            assert_eq!(
                t,
                dummy_env.msm_limbs[i],
                "{}",
                format_args!(
                    "MSM limb {i}. Exp value is {:?}, computed is {:?}",
                    t.to_biguint(),
                    dummy_env.msm_limbs[i].to_biguint()
                )
            )
        }
    }

    #[test]
    fn test_decomposition_zero() {
        test_decomposition_generic(Fp::zero());
    }

    #[test]
    fn test_decomposition_one() {
        test_decomposition_generic(Fp::one());
    }

    #[test]
    fn test_decomposition_random_first_limb_only() {
        let mut rng = make_test_rng();
        let x = rng.gen_range(0..2u128.pow(88) - 1);
        test_decomposition_generic(Fp::from(x));
    }

    #[test]
    fn test_decomposition_second_limb_only() {
        test_decomposition_generic(Fp::from(2u128.pow(88)));
        test_decomposition_generic(Fp::from(2u128.pow(88) + 1));
        test_decomposition_generic(Fp::from(2u128.pow(88) + 2));
        test_decomposition_generic(Fp::from(2u128.pow(88) + 16));
        test_decomposition_generic(Fp::from(2u128.pow(88) + 23234));
    }

    #[test]
    fn test_decomposition_random_second_limb_only() {
        let mut rng = make_test_rng();
        let x = rng.gen_range(0..2u128.pow(88) - 1);
        test_decomposition_generic(Fp::from(2u128.pow(88) + x));
    }

    #[test]
    fn test_decomposition_random() {
        let mut rng = make_test_rng();
        test_decomposition_generic(Fp::rand(&mut rng));
    }

    #[test]
    fn test_decomposition_order_minus_one() {
        let x = BigUint::from_bytes_be(&<Fp as PrimeField>::Params::MODULUS.to_bytes_be())
            - BigUint::from_str("1").unwrap();

        test_decomposition_generic(Fp::from(x));
    }
}
