use std::marker::PhantomData;

use crate::{
    circuits::{constraints::ConstraintSystem, gate::CircuitGate, polynomial::COLUMNS},
    curve::KimchiCurve,
    groupmap::GroupMap,
    mina_poseidon::FqSponge,
    plonk_sponge::FrSponge,
    poly_commitment::srs::SRS,
    proof::ProverProof,
    prover_index::ProverIndex,
    verifier::verify,
    verifier_index::VerifierIndex,
};
use ark_ec::AffineCurve;
use ark_ff::{PrimeField, Zero as _};
use poly_commitment::commitment::CommitmentCurve;

use super::{checked_runner::RunState, traits::SnarkyType};

// TODO: implement digest function
pub struct CompiledCircuit<Circuit>
where
    Circuit: SnarkyCircuit,
{
    circuit: Circuit,
    sys: RunState<ScalarField<Circuit::Curve>>,
    public_input_size: usize,
    pub gates: Vec<CircuitGate<ScalarField<Circuit::Curve>>>,
    phantom: PhantomData<Circuit>,
}

#[derive(Debug)]
pub struct Witness<F>(pub [Vec<F>; COLUMNS]);

// alias
type ScalarField<C> = <C as AffineCurve>::ScalarField;
type BaseField<C> = <C as AffineCurve>::BaseField;

pub struct ProverIndexWrapper<Circuit>
where
    Circuit: SnarkyCircuit,
{
    compiled_circuit: CompiledCircuit<Circuit>,
    index: ProverIndex<Circuit::Curve>,
}

impl<Circuit> ProverIndexWrapper<Circuit>
where
    Circuit: SnarkyCircuit,
{
    /// Produces an assembly-like encoding of the circuit.
    pub fn asm(&self) -> String {
        crate::circuits::gate::Circuit::new(
            self.compiled_circuit.public_input_size,
            &self.compiled_circuit.gates,
        )
        .generate_asm()
    }

    /// Produces a proof for the given public input.
    pub fn prove<EFqSponge, EFrSponge>(
        // TODO: this should not be mutable ideally
        &mut self,
        public_input: <Circuit::PublicInput as SnarkyType<ScalarField<Circuit::Curve>>>::OutOfCircuit,
        debug: bool,
    ) -> (
        ProverProof<Circuit::Curve>,
        <Circuit::PublicOutput as SnarkyType<ScalarField<Circuit::Curve>>>::OutOfCircuit,
    )
    where
        <Circuit::Curve as AffineCurve>::BaseField: PrimeField,
        EFqSponge: Clone
            + FqSponge<BaseField<Circuit::Curve>, Circuit::Curve, ScalarField<Circuit::Curve>>,
        EFrSponge: FrSponge<ScalarField<Circuit::Curve>>,
    {
        // create public input
        let mut public_input_and_output =
            Circuit::PublicInput::value_to_field_elements(&public_input).0;

        // pad with 0s with the public output for now
        public_input_and_output.resize(
            Circuit::PublicInput::SIZE_IN_FIELD_ELEMENTS
                + Circuit::PublicOutput::SIZE_IN_FIELD_ELEMENTS,
            ScalarField::<Circuit::Curve>::zero(),
        );

        dbg!("yo");
        // init
        self.compiled_circuit
            .sys
            .generate_witness_init(public_input_and_output.clone());

        dbg!("yo");
        // run circuit and get return var
        let public_input_var: Circuit::PublicInput = self.compiled_circuit.sys.public_input();
        let return_var = self
            .compiled_circuit
            .circuit
            .circuit(&mut self.compiled_circuit.sys, public_input_var);

        dbg!("yo");
        // get values from private input vec
        let (return_cvars, aux) = return_var.to_cvars();
        let public_output_values = self.compiled_circuit.sys.public_output_values(return_cvars);

        dbg!("yo");
        // create constraint between public output var and return var
        self.compiled_circuit.sys.wire_public_output(return_var);

        dbg!("yo");
        // finalize
        let mut witness = self.compiled_circuit.sys.generate_witness();

        dbg!("yo");
        // replace public output part of witness
        let start = Circuit::PublicInput::SIZE_IN_FIELD_ELEMENTS;
        let end = start + Circuit::PublicOutput::SIZE_IN_FIELD_ELEMENTS;
        for (cell, val) in &mut witness.0[0][start..end]
            .iter_mut()
            .zip(&public_output_values)
        {
            *cell = *val;
        }

        dbg!("yo");
        // same but with the full public input
        for (cell, val) in &mut public_input_and_output[start..end]
            .iter_mut()
            .zip(&public_output_values)
        {
            *cell = *val;
        }

        dbg!("yo");
        // reconstruct public output
        let public_output =
            Circuit::PublicOutput::value_of_field_elements(public_output_values, aux);

        witness.debug();

        dbg!("yo");
        // verify the witness
        if debug {
            self.index
                .verify(&witness.0, &public_input_and_output)
                .unwrap();
        }

        // produce a proof
        let group_map = <Circuit::Curve as CommitmentCurve>::Map::setup();

        let proof: ProverProof<Circuit::Curve> =
            ProverProof::create::<EFqSponge, EFrSponge>(&group_map, witness.0, &[], &self.index)
                .unwrap();

        // return proof + public output
        (proof, public_output)
    }
}

pub struct VerifierIndexWrapper<Circuit>
where
    Circuit: SnarkyCircuit,
{
    index: VerifierIndex<Circuit::Curve>,
}

impl<Circuit> VerifierIndexWrapper<Circuit>
where
    Circuit: SnarkyCircuit,
{
    pub fn verify<EFqSponge, EFrSponge>(
        &self,
        proof: ProverProof<Circuit::Curve>,
        public_input: <Circuit::PublicInput as SnarkyType<ScalarField<Circuit::Curve>>>::OutOfCircuit,
        public_output: <Circuit::PublicOutput as SnarkyType<ScalarField<Circuit::Curve>>>::OutOfCircuit,
    ) where
        <Circuit::Curve as AffineCurve>::BaseField: PrimeField,
        EFqSponge: Clone
            + FqSponge<BaseField<Circuit::Curve>, Circuit::Curve, ScalarField<Circuit::Curve>>,
        EFrSponge: FrSponge<ScalarField<Circuit::Curve>>,
    {
        let mut public_input = Circuit::PublicInput::value_to_field_elements(&public_input).0;
        public_input.extend(Circuit::PublicOutput::value_to_field_elements(&public_output).0);

        // verify the proof
        let group_map = <Circuit::Curve as CommitmentCurve>::Map::setup();

        verify::<Circuit::Curve, EFqSponge, EFrSponge>(
            &group_map,
            &self.index,
            &proof,
            &public_input,
        )
        .unwrap()
    }
}

fn compile<Circuit: SnarkyCircuit>(circuit: Circuit) -> CompiledCircuit<Circuit> {
    // calculate public input size
    let public_input_size = Circuit::PublicInput::SIZE_IN_FIELD_ELEMENTS
        + Circuit::PublicOutput::SIZE_IN_FIELD_ELEMENTS;

    // create snarky constraint system
    let mut sys = RunState::new::<Circuit::Curve>(
        Circuit::PublicInput::SIZE_IN_FIELD_ELEMENTS,
        Circuit::PublicOutput::SIZE_IN_FIELD_ELEMENTS,
        true,
    );

    // run circuit and get return var
    let public_input: Circuit::PublicInput = sys.public_input();
    let return_var = circuit.circuit(&mut sys, public_input);

    // create constraint between public output var and return var
    sys.wire_public_output(return_var);

    // compile to gates
    let gates = sys.compile().to_vec();

    // turn as_prover
    sys.as_prover = true;

    // return compiled circuit
    CompiledCircuit {
        circuit,
        sys,
        public_input_size,
        gates,
        phantom: PhantomData,
    }
}

pub trait SnarkyCircuit: Sized {
    type Curve: KimchiCurve;

    type PublicInput: SnarkyType<ScalarField<Self::Curve>>;
    type PublicOutput: SnarkyType<ScalarField<Self::Curve>>;

    fn circuit(
        &self,
        sys: &mut RunState<ScalarField<Self::Curve>>,
        public_input: Self::PublicInput,
    ) -> Self::PublicOutput;

    fn compile_to_indexes(self) -> (ProverIndexWrapper<Self>, VerifierIndexWrapper<Self>)
    where
        <Self::Curve as AffineCurve>::BaseField: PrimeField,
    {
        let compiled_circuit = compile(self);

        // create constraint system
        let cs = ConstraintSystem::create(compiled_circuit.gates.clone())
            .public(compiled_circuit.public_input_size)
            .build()
            .unwrap();

        // create SRS (for vesta, as the circuit is in Fp)
        let mut srs = SRS::<Self::Curve>::create(cs.domain.d1.size as usize);
        srs.add_lagrange_basis(cs.domain.d1);
        let srs = std::sync::Arc::new(srs);

        println!("using an SRS of size {}", srs.g.len());

        // create indexes
        let (endo_q, _endo_r) =
            <<Self as SnarkyCircuit>::Curve as KimchiCurve>::OtherCurve::endos();

        let prover_index =
            crate::prover_index::ProverIndex::<Self::Curve>::create(cs, *endo_q, srs);
        let verifier_index = prover_index.verifier_index();

        let prover_index = ProverIndexWrapper {
            compiled_circuit,
            index: prover_index,
        };

        let verifier_index = VerifierIndexWrapper {
            index: verifier_index,
        };

        (prover_index, verifier_index)
    }
}
