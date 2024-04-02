use ark_ff::UniformRand;

use kimchi::circuits::domains::EvaluationDomains;
use poly_commitment::pairing_proof::PairingSRS;

use kimchi_msm::{
    columns::Column,
    ffa::{
        columns::{FFA_NPUB_COLUMNS, FFA_N_COLUMNS},
        constraint::ConstraintBuilderEnv as FFAConstraintBuilderEnv,
        interpreter::{self as ffa_interpreter, FFAInterpreterEnv},
        witness::WitnessBuilderEnv as FFAWitnessBuilderEnv,
    },
    lookups::LookupTableIDs,
    precomputed_srs::get_bn254_srs,
    prover::prove,
    verifier::verify,
    BaseSponge, Ff1, Fp, OpeningProof, ScalarSponge, BN254,
};

pub fn main() {
    // FIXME: use a proper RNG
    let mut rng = o1_utils::tests::make_test_rng();

    println!("Creating the domain and SRS");
    let domain_size = 1 << 8;
    let domain = EvaluationDomains::<Fp>::create(domain_size).unwrap();

    let srs: PairingSRS<BN254> = get_bn254_srs(domain);

    let mut witness_env = FFAWitnessBuilderEnv::<Fp>::empty();
    let mut constraint_env = FFAConstraintBuilderEnv::<Fp>::empty();

    ffa_interpreter::constrain_ff_addition(&mut constraint_env);

    let row_num = 10;
    assert!(row_num <= domain_size);

    for _row_i in 0..row_num {
        println!("processing row {_row_i:?}");
        let a: Ff1 = Ff1::rand(&mut rng);
        let b: Ff1 = Ff1::rand(&mut rng);

        //use rand::Rng;
        //let a: Ff1 = From::from(rng.gen_range(0..(1 << 50)));
        //let b: Ff1 = From::from(rng.gen_range(0..(1 << 50)));
        ffa_interpreter::ff_addition_circuit(&mut witness_env, a, b);
        witness_env.next_row();
    }

    let inputs = witness_env.get_witness(domain_size);
    let pub_inputs = inputs.evaluations.to_pub_columns::<FFA_NPUB_COLUMNS>();
    let constraints = constraint_env.constraints;

    println!("Generating the proof");
    let proof = prove::<
        _,
        OpeningProof,
        BaseSponge,
        ScalarSponge,
        Column,
        _,
        FFA_N_COLUMNS,
        LookupTableIDs,
    >(domain, &srs, &constraints, inputs, &mut rng)
    .unwrap();

    println!("Verifying the proof");
    let verifies = verify::<
        _,
        OpeningProof,
        BaseSponge,
        ScalarSponge,
        FFA_N_COLUMNS,
        FFA_NPUB_COLUMNS,
        LookupTableIDs,
    >(domain, &srs, &constraints, &proof, pub_inputs);
    println!("Proof verification result: {verifies}")
}
