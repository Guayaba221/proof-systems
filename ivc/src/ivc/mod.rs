pub mod columns;
pub mod constraints;
pub mod helpers;
pub mod interpreter;
pub mod lookups;

use self::columns::N_BLOCKS;
use crate::poseidon_8_56_5_3_2::bn254::NB_CONSTRAINTS as IVC_POSEIDON_NB_CONSTRAINTS;

/// The biggest packing variant for foreign field. Used for hashing. 150-bit limbs.
pub const LIMB_BITSIZE_XLARGE: usize = 150;

/// The biggest packing format, 2 limbs.
pub const N_LIMBS_XLARGE: usize = 2;

/// Number of additional columns that a reduction to degree 2 will
/// require.
// This value has been generated using a fake folding config like in
// [folding::tests::test_quadraticization]
pub const N_ADDITIONAL_WIT_COL_QUAD: usize = 48;

/// Number of challenges in the IVC circuit.
/// It is the maximum number of constraints per row.
/// We do suppose the Poseidon circuit has the highest number of constraints,
/// for now.
/// We also add the number of "blocks" in the IVC circuit as there will be
/// [N_BLOCKS] alphas required to aggregate all blocks on each row.
pub const IVC_NB_CHALLENGES: usize = IVC_POSEIDON_NB_CONSTRAINTS + N_BLOCKS;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        ivc::{
            columns::{IVCColumn, IVC_NB_TOTAL_FIXED_SELECTORS, N_BLOCKS},
            constraints::constrain_ivc,
            interpreter::{build_selectors, ivc_circuit},
            lookups::IVCLookupTable,
        },
        poseidon_8_56_5_3_2::{
            bn254::{PoseidonBN254Parameters, STATE_SIZE as IVC_POSEIDON_STATE_SIZE},
            interpreter::PoseidonParams,
        },
    };
    use ark_ff::{Field, UniformRand, Zero};
    use kimchi_msm::{
        circuit_design::{
            composition::{IdMPrism, MPrism},
            ConstraintBuilderEnv, SubEnvLookup, WitnessBuilderEnv,
        },
        columns::ColumnIndexer,
        logup::LookupTableID,
        Ff1, Fp,
    };
    use o1_utils::box_array;
    use rand::{CryptoRng, RngCore};

    use super::IVC_NB_CHALLENGES;

    // Total number of columns in IVC and Application circuits.
    pub const TEST_N_COL_TOTAL: usize = IVCColumn::N_COL + 50;

    // We do not have any additional constraint that the IVC circuit
    pub const TEST_N_CHALS: usize = IVC_NB_CHALLENGES;

    pub const TEST_DOMAIN_SIZE: usize = 1 << 15;

    type IVCWitnessBuilderEnvRaw<LT> = WitnessBuilderEnv<
        Fp,
        IVCColumn,
        { <IVCColumn as ColumnIndexer>::N_COL - N_BLOCKS },
        { <IVCColumn as ColumnIndexer>::N_COL - N_BLOCKS },
        0,
        IVC_NB_TOTAL_FIXED_SELECTORS,
        LT,
    >;

    /// Generic IVC circuit builder.
    fn build_ivc_circuit<
        RNG: RngCore + CryptoRng,
        LT: LookupTableID,
        L: MPrism<Source = LT, Target = IVCLookupTable<Ff1>>,
    >(
        rng: &mut RNG,
        domain_size: usize,
        fold_iteration: usize,
        lt_lens: L,
    ) -> IVCWitnessBuilderEnvRaw<LT> {
        let mut witness_env = IVCWitnessBuilderEnvRaw::<LT>::create();

        let mut comms_left: Box<_> = box_array![(Ff1::zero(),Ff1::zero()); TEST_N_COL_TOTAL];
        let mut comms_right: Box<_> = box_array![(Ff1::zero(),Ff1::zero()); TEST_N_COL_TOTAL];
        let mut comms_output: Box<_> = box_array![(Ff1::zero(),Ff1::zero()); TEST_N_COL_TOTAL];

        for i in 0..TEST_N_COL_TOTAL {
            comms_left[i] = (
                <Ff1 as UniformRand>::rand(rng),
                <Ff1 as UniformRand>::rand(rng),
            );
            comms_right[i] = (
                <Ff1 as UniformRand>::rand(rng),
                <Ff1 as UniformRand>::rand(rng),
            );
            comms_output[i] = (
                <Ff1 as UniformRand>::rand(rng),
                <Ff1 as UniformRand>::rand(rng),
            );
        }

        println!("Building fixed selectors");
        let mut fixed_selectors: Vec<Vec<Fp>> =
            build_selectors::<TEST_N_COL_TOTAL, TEST_N_CHALS>(domain_size).to_vec();

        // Write constants
        {
            let rc = PoseidonBN254Parameters.constants();
            rc.iter().enumerate().for_each(|(round, rcs)| {
                rcs.iter().enumerate().for_each(|(state_index, rc)| {
                    let rc = vec![*rc; TEST_DOMAIN_SIZE];
                    fixed_selectors[N_BLOCKS + round * IVC_POSEIDON_STATE_SIZE + state_index] = rc;
                });
            });
        }

        witness_env.set_fixed_selectors(fixed_selectors);

        let alpha = <Fp as UniformRand>::rand(rng);
        let alphas: Vec<_> = (0..TEST_N_CHALS).map(|i| alpha.pow([i as u64])).collect();

        println!("Calling the IVC circuit");
        // TODO add nonzero E/T values.
        ivc_circuit::<_, _, _, _, TEST_N_COL_TOTAL, TEST_N_CHALS>(
            &mut SubEnvLookup::new(&mut witness_env, lt_lens),
            fold_iteration,
            comms_left,
            comms_right,
            comms_output,
            [(
                <Ff1 as UniformRand>::rand(rng),
                <Ff1 as UniformRand>::rand(rng),
            ); 3],
            [(
                <Ff1 as UniformRand>::rand(rng),
                <Ff1 as UniformRand>::rand(rng),
            ); 2],
            <Fp as UniformRand>::rand(rng),
            Box::new((*alphas.into_boxed_slice()).try_into().unwrap()),
            &PoseidonBN254Parameters,
            TEST_DOMAIN_SIZE,
        );

        witness_env
    }

    #[test]
    /// Tests if building the IVC circuit succeeds when using the general case
    /// (i.e. fold_iteration != 0).
    pub fn test_ivc_circuit_general_case() {
        let mut rng = o1_utils::tests::make_test_rng(None);
        build_ivc_circuit::<_, IVCLookupTable<Ff1>, _>(
            &mut rng,
            1 << 15,
            1,
            IdMPrism::<IVCLookupTable<Ff1>>::default(),
        );
    }

    #[test]
    fn test_regression_ivc_constraints() {
        let mut constraint_env = ConstraintBuilderEnv::<Fp, IVCLookupTable<Ff1>>::create();
        constrain_ivc::<Ff1, _>(&mut constraint_env);
        let constraints = constraint_env.get_relation_constraints();

        let mut constraints_degrees = HashMap::new();

        // Regression testing for the number of constraints and their degree
        {
            // 55 + 432 (Poseidon)
            assert_eq!(constraints.len(), 487);
            constraints.iter().for_each(|c| {
                let degree = c.degree(1, 0);
                *constraints_degrees.entry(degree).or_insert(0) += 1;
            });

            assert_eq!(constraints_degrees.get(&1), None);
            assert_eq!(constraints_degrees.get(&2), Some(&6));
            assert_eq!(constraints_degrees.get(&3), Some(&215));
            assert_eq!(constraints_degrees.get(&4), Some(&245));
            assert_eq!(constraints_degrees.get(&5), Some(&21));

            // Maximum degree is 5
            // - fold_iteration increases by one
            // - the public selectors increase by one
            assert!(constraints.iter().all(|c| c.degree(1, 0) <= 5));
        }
    }

    #[test]
    /// Completeness test for the IVC circuit in the general case (i.e.
    /// fold_iteration != 0).
    fn test_completeness_ivc_general_case() {
        let fold_iteration = 1;

        let mut rng = o1_utils::tests::make_test_rng(None);

        let domain_size = 1 << 15;

        let witness_env = build_ivc_circuit::<_, IVCLookupTable<Ff1>, _>(
            &mut rng,
            domain_size,
            fold_iteration,
            IdMPrism::<IVCLookupTable<Ff1>>::default(),
        );
        let relation_witness = witness_env.get_relation_witness(domain_size);

        let mut constraint_env = ConstraintBuilderEnv::<Fp, IVCLookupTable<Ff1>>::create();
        constrain_ivc::<Ff1, _>(&mut constraint_env);
        let constraints = constraint_env.get_relation_constraints();

        let mut fixed_selectors: Box<[Vec<Fp>; IVC_NB_TOTAL_FIXED_SELECTORS]> = {
            Box::new(build_selectors::<TEST_N_COL_TOTAL, TEST_N_CHALS>(
                domain_size,
            ))
        };

        // Write constants
        {
            let rc = PoseidonBN254Parameters.constants();
            rc.iter().enumerate().for_each(|(round, rcs)| {
                rcs.iter().enumerate().for_each(|(state_index, rc)| {
                    let rc = vec![*rc; domain_size];
                    fixed_selectors[N_BLOCKS + round * IVC_POSEIDON_STATE_SIZE + state_index] = rc;
                });
            });
        }

        kimchi_msm::test::test_completeness_generic_no_lookups::<
            { IVCColumn::N_COL - N_BLOCKS },
            { IVCColumn::N_COL - N_BLOCKS },
            0,
            IVC_NB_TOTAL_FIXED_SELECTORS,
            _,
        >(
            constraints,
            fixed_selectors,
            relation_witness,
            domain_size,
            &mut rng,
        );
    }

    #[test]
    /// Completeness test for the IVC circuit in the base case (i.e.
    /// fold_iteration = 0).
    fn test_completeness_ivc_base_case() {
        let fold_iteration = 0;

        let mut rng = o1_utils::tests::make_test_rng(None);

        let domain_size = 1 << 15;

        let witness_env = build_ivc_circuit::<_, IVCLookupTable<Ff1>, _>(
            &mut rng,
            domain_size,
            fold_iteration,
            IdMPrism::<IVCLookupTable<Ff1>>::default(),
        );
        let relation_witness = witness_env.get_relation_witness(domain_size);

        let mut constraint_env = ConstraintBuilderEnv::<Fp, IVCLookupTable<Ff1>>::create();
        constrain_ivc::<Ff1, _>(&mut constraint_env);
        let constraints = constraint_env.get_relation_constraints();

        let mut fixed_selectors: Box<[Vec<Fp>; IVC_NB_TOTAL_FIXED_SELECTORS]> = {
            Box::new(build_selectors::<TEST_N_COL_TOTAL, TEST_N_CHALS>(
                domain_size,
            ))
        };

        // Write constants
        {
            let rc = PoseidonBN254Parameters.constants();
            rc.iter().enumerate().for_each(|(round, rcs)| {
                rcs.iter().enumerate().for_each(|(state_index, rc)| {
                    let rc = vec![*rc; domain_size];
                    fixed_selectors[N_BLOCKS + round * IVC_POSEIDON_STATE_SIZE + state_index] = rc;
                });
            });
        }

        kimchi_msm::test::test_completeness_generic_no_lookups::<
            { IVCColumn::N_COL - N_BLOCKS },
            { IVCColumn::N_COL - N_BLOCKS },
            0,
            IVC_NB_TOTAL_FIXED_SELECTORS,
            _,
        >(
            constraints,
            fixed_selectors,
            relation_witness,
            domain_size,
            &mut rng,
        );
    }
}
