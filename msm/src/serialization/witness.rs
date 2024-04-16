use ark_ff::{FpParameters, PrimeField};
use num_bigint::BigUint;
use o1_utils::FieldHelpers;

use crate::{
    columns::{Column, ColumnIndexer},
    serialization::{
        column::{SerializationColumn, SER_N_COLUMNS},
        interpreter::InterpreterEnv,
        Lookup, LookupTable,
    },
    witness::Witness,
    LIMB_BITSIZE, N_LIMBS,
};
use kimchi::circuits::domains::EvaluationDomains;
use std::{collections::BTreeMap, iter};

// TODO The parameter `Fp` clashes with the `Fp` type alias in the lib. Rename this into `F.`
// TODO `WitnessEnv`
/// Environment for the serializer interpreter
pub struct Env<Fp> {
    /// Single-row witness columns, in raw form. For accessing [`Witness`], see the
    /// `get_witness` method.
    pub witness: Witness<SER_N_COLUMNS, Fp>,

    /// Keep track of the RangeCheck4 table multiplicities.
    /// The value `0` is used as a (repeated) dummy value.
    // Boxing to avoid stack overflow
    pub lookup_t_multiplicities_rangecheck4: Box<[Fp; 1 << 4]>,

    /// Keep track of the lookup multiplicities.
    pub lookup_multiplicities: BTreeMap<LookupTable, Vec<Fp>>,

    /// Keep track of the lookups for each row.
    pub lookups: BTreeMap<LookupTable, Vec<Lookup<Fp>>>,
}

// TODO The parameter `Fp` clashes with the `Fp` type alias in the lib. Rename this into `F.`
impl<F: PrimeField> InterpreterEnv<F> for Env<F> {
    type Position = Column;

    // Requiring an F element as we would need to compute values up to 180 bits
    // in the 15 bits decomposition.
    type Variable = F;

    fn add_constraint(&mut self, cst: Self::Variable) {
        assert_eq!(cst, F::zero());
    }

    fn constant(value: F) -> Self::Variable {
        value
    }

    fn get_column(pos: SerializationColumn) -> Self::Position {
        pos.to_column()
    }

    fn read_column(&self, ix: Column) -> Self::Variable {
        let Column::X(i) = ix else { todo!() };
        self.witness.cols[i]
    }

    fn range_check_abs15bit(&mut self, value: &Self::Variable) {
        assert!(*value < F::from(1u64 << 15) || *value >= F::zero() - F::from(1u64 << 15));
        // TODO implement actual lookups
    }

    fn range_check_abs4bit(&mut self, value: &Self::Variable) {
        assert!(*value < F::from(1u64 << 4) || *value >= F::zero() - F::from(1u64 << 4));
        // TODO implement actual lookups
    }

    fn range_check_ff_highest<Ff: PrimeField>(&mut self, value: &Self::Variable) {
        let f_bui: BigUint = TryFrom::try_from(Ff::Params::MODULUS).unwrap();
        let top_modulus: BigUint = f_bui >> ((N_LIMBS - 1) * LIMB_BITSIZE);
        let top_modulus_f: F = F::from_biguint(&top_modulus).unwrap();
        assert!(
            *value < top_modulus_f,
            "The value {:?} was higher than modulus {:?}",
            (*value).to_bigint_positive(),
            top_modulus_f.to_bigint_positive()
        );
    }

    fn range_check15(&mut self, value: &Self::Variable) {
        let value_biguint = value.to_biguint();
        assert!(value_biguint < BigUint::from(2u128.pow(15)));
        // Adding multiplicities
        let value_usize: usize = value_biguint.clone().try_into().unwrap();
        self.lookup_multiplicities
            .get_mut(&LookupTable::RangeCheck15)
            .unwrap()[value_usize] += F::one();
        self.lookups
            .get_mut(&LookupTable::RangeCheck15)
            .unwrap()
            .push(Lookup {
                table_id: LookupTable::RangeCheck15,
                numerator: F::one(),
                value: vec![*value],
            })
    }

    fn range_check4(&mut self, value: &Self::Variable) {
        let value_biguint = value.to_biguint();
        assert!(value_biguint < BigUint::from(2u128.pow(4)));
        // Adding multiplicities
        let value_usize: usize = value_biguint.clone().try_into().unwrap();
        self.lookup_multiplicities
            .get_mut(&LookupTable::RangeCheck4)
            .unwrap()[value_usize] += F::one();
        self.lookups
            .get_mut(&LookupTable::RangeCheck4)
            .unwrap()
            .push(Lookup {
                table_id: LookupTable::RangeCheck4,
                numerator: F::one(),
                value: vec![*value],
            })
    }

    fn copy(&mut self, x: &Self::Variable, position: Self::Position) -> Self::Variable {
        self.write_column(position, *x);
        *x
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
        let res_fp: F = res.into();
        self.write_column(position, res_fp);
        res_fp
    }
}

impl<Fp: PrimeField> Env<Fp> {
    pub fn write_column(&mut self, position: Column, value: Fp) {
        match position {
            Column::X(i) => self.witness.cols[i] = value,
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

    pub fn reset(&mut self) {
        *self.lookups.get_mut(&LookupTable::RangeCheck4).unwrap() = Vec::new();
        *self.lookups.get_mut(&LookupTable::RangeCheck15).unwrap() = Vec::new();
    }

    pub fn get_rangecheck4_multipliticies(&self, domain: EvaluationDomains<Fp>) -> Vec<Fp> {
        let mut m = Vec::with_capacity(domain.d1.size as usize);
        m.extend(self.lookup_multiplicities[&LookupTable::RangeCheck4].to_vec());
        let repeated_dummy_value: Vec<Fp> = iter::repeat(-Fp::zero())
            .take((domain.d1.size - (1 << 4)) as usize)
            .collect();
        m.extend(repeated_dummy_value);
        assert_eq!(m.len(), domain.d1.size as usize);
        m
    }
    pub fn get_rangecheck15_multipliticies(&self, domain: EvaluationDomains<Fp>) -> Vec<Fp> {
        assert_eq!(domain.d1.size, 1 << 15);
        self.lookup_multiplicities[&LookupTable::RangeCheck15].to_vec()
    }
}

impl<Fp: PrimeField> Env<Fp> {
    pub fn create() -> Self {
        let mut lookups = BTreeMap::new();
        lookups.insert(LookupTable::RangeCheck4, Vec::new());
        lookups.insert(LookupTable::RangeCheck15, Vec::new());

        let mut lookup_multiplicities = BTreeMap::new();
        lookup_multiplicities.insert(LookupTable::RangeCheck4, vec![Fp::zero(); 1 << 4]);
        lookup_multiplicities.insert(LookupTable::RangeCheck15, vec![Fp::zero(); 1 << 15]);

        Self {
            witness: Witness {
                cols: Box::new([Fp::zero(); SER_N_COLUMNS]),
            },

            lookup_multiplicities,
            lookup_t_multiplicities_rangecheck4: Box::new([Fp::zero(); 1 << 4]),
            lookups,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{serialization::N_INTERMEDIATE_LIMBS, LIMB_BITSIZE, N_LIMBS};

    use super::Env;
    use crate::serialization::{
        column::SerializationColumn,
        interpreter::{deserialize_field_element, InterpreterEnv},
    };
    use ark_ff::{BigInteger, FpParameters as _, One, PrimeField, UniformRand, Zero};
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
        let limbs_to_assert = [limb0, limb1, limb2];
        for (i, limb) in limbs_to_assert.iter().enumerate() {
            assert_eq!(
                Fp::from(*limb),
                dummy_env.read_column_direct(SerializationColumn::ChalKimchi(i))
            );
        }

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
                let intermediate_v =
                    dummy_env.read_column_direct(SerializationColumn::ChalIntermediate(j));
                assert_eq!(
                    t,
                    intermediate_v,
                    "{}",
                    format_args!(
                        "Intermediate limb {j}. Exp value is {:?}, computed is {:?}",
                        t.to_biguint(),
                        intermediate_v.to_biguint()
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
            let converted_v = dummy_env.read_column_direct(SerializationColumn::ChalConverted(i));
            assert_eq!(
                t,
                converted_v,
                "{}",
                format_args!(
                    "MSM limb {i}. Exp value is {:?}, computed is {:?}",
                    t.to_biguint(),
                    converted_v.to_biguint()
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
