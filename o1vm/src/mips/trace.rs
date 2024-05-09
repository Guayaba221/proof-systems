use crate::{
    folding::ScalarField,
    mips::{
        column::{N_MIPS_COLS, N_MIPS_REL_COLS, N_MIPS_SEL_COLS},
        constraints::Env,
        interpreter::{interpret_instruction, Instruction},
    },
    trace::{DecomposableTrace, DecomposableTracer},
};
use ark_ff::Zero;
use kimchi_msm::witness::Witness;
use std::{array, collections::BTreeMap};
use strum::IntoEnumIterator;

use super::folding::MIPSFoldingConfig;

/// The MIPS circuit trace
pub type MIPSTrace =
    DecomposableTrace<N_MIPS_COLS, N_MIPS_REL_COLS, N_MIPS_SEL_COLS, MIPSFoldingConfig>;

impl
    DecomposableTracer<
        N_MIPS_COLS,
        N_MIPS_REL_COLS,
        N_MIPS_SEL_COLS,
        MIPSFoldingConfig,
        Env<ScalarField<MIPSFoldingConfig>>,
    > for MIPSTrace
{
    fn new(domain_size: usize, env: &mut Env<ScalarField<MIPSFoldingConfig>>) -> Self {
        let mut circuit = Self {
            domain_size,
            witness: BTreeMap::new(),
            constraints: Default::default(),
            lookups: Default::default(),
        };

        for instr in Instruction::iter().flat_map(|x| x.into_iter()) {
            circuit.witness.insert(
                instr,
                Witness {
                    cols: Box::new(std::array::from_fn(|_| Vec::with_capacity(domain_size))),
                },
            );
            interpret_instruction(env, instr);
            circuit.constraints.insert(instr, env.constraints.clone());
            circuit.lookups.insert(instr, env.lookups.clone());
            env.scratch_state_idx = 0; // Reset the scratch state index for the next instruction
            env.constraints = vec![]; // Clear the constraints for the next instruction
            env.lookups = vec![]; // Clear the lookups for the next instruction
        }
        circuit
    }

    fn push_row(
        &mut self,
        opcode: Instruction,
        row: &[ScalarField<MIPSFoldingConfig>; N_MIPS_REL_COLS],
    ) {
        self.witness.entry(opcode).and_modify(|wit| {
            for (i, value) in row.iter().enumerate() {
                if wit.cols[i].len() < wit.cols[i].capacity() {
                    wit.cols[i].push(*value);
                }
            }
        });
    }

    fn pad_with_row(
        &mut self,
        opcode: Instruction,
        row: &[ScalarField<MIPSFoldingConfig>; N_MIPS_REL_COLS],
    ) -> usize {
        let len = self.witness[&opcode].cols[0].len();
        assert!(len <= self.domain_size);
        let rows_to_add = self.domain_size - len;
        for _ in 0..rows_to_add {
            self.push_row(opcode, row);
        }
        rows_to_add
    }

    fn pad_with_zeros(&mut self, opcode: Instruction) -> usize {
        let len = self.witness[&opcode].cols[0].len();
        assert!(len <= self.domain_size);
        let rows_to_add = self.domain_size - len;
        self.witness.entry(opcode).and_modify(|wit| {
            for col in wit.cols.iter_mut() {
                col.extend((0..rows_to_add).map(|_| ScalarField::<MIPSFoldingConfig>::zero()));
            }
        });
        rows_to_add
    }

    fn pad_dummy(&mut self, opcode: Instruction) -> usize {
        if !self.in_circuit(opcode) {
            0
        } else {
            let row = array::from_fn(|i| self.witness[&opcode].cols[i][0]);
            self.pad_with_row(opcode, &row)
        }
    }

    fn pad_witnesses(&mut self) {
        for step in Instruction::iter().flat_map(|step| step.into_iter()) {
            self.pad_dummy(step);
        }
    }
}
