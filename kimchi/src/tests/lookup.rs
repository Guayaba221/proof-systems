use super::framework::TestFramework;
use crate::circuits::{
    gate::{CircuitGate, GateType},
    lookup::tables::LookupTable,
    wires::Wire,
};
use ark_ff::Zero;
use array_init::array_init;
use mina_curves::pasta::fp::Fp;

fn setup_lookup_proof(use_values_from_table: bool, num_lookups: usize, table_sizes: Vec<usize>) {
    let lookup_table_values: Vec<Vec<_>> = table_sizes
        .iter()
        .map(|size| (0..*size).map(|_| rand::random()).collect())
        .collect();
    let lookup_tables = lookup_table_values
        .iter()
        .enumerate()
        .map(|(id, lookup_table_values)| {
            let index_column = (0..lookup_table_values.len() as u64)
                .map(Into::into)
                .collect();
            LookupTable {
                id: id as i32,
                data: vec![index_column, lookup_table_values.clone()],
            }
        })
        .collect();

    // circuit gates
    let gates = (0..num_lookups)
        .map(|i| CircuitGate {
            typ: GateType::Lookup,
            coeffs: vec![],
            wires: Wire::new(i),
        })
        .collect();

    let witness = {
        let mut lookup_table_ids = Vec::with_capacity(num_lookups);
        let mut lookup_indexes: [_; 3] = array_init(|_| Vec::with_capacity(num_lookups));
        let mut lookup_values: [_; 3] = array_init(|_| Vec::with_capacity(num_lookups));
        let unused = || vec![Fp::zero(); num_lookups];

        let num_tables = table_sizes.len();
        let mut tables_used = std::collections::HashSet::new();
        for _ in 0..num_lookups {
            let table_id = rand::random::<usize>() % num_tables;
            tables_used.insert(table_id);
            let lookup_table_values: &Vec<Fp> = &lookup_table_values[table_id];
            lookup_table_ids.push((table_id as u64).into());
            for i in 0..3 {
                let index = rand::random::<usize>() % lookup_table_values.len();
                let value = if use_values_from_table {
                    lookup_table_values[index]
                } else {
                    rand::random()
                };
                lookup_indexes[i].push((index as u64).into());
                lookup_values[i].push(value);
            }
        }

        // Sanity check: if we were given multiple tables, we should have used multiple tables,
        // assuming we're generating enough gates.
        assert!(tables_used.len() >= 2 || table_sizes.len() <= 1);

        let [lookup_indexes0, lookup_indexes1, lookup_indexes2] = lookup_indexes;
        let [lookup_values0, lookup_values1, lookup_values2] = lookup_values;
        [
            lookup_table_ids,
            lookup_indexes0,
            lookup_values0,
            lookup_indexes1,
            lookup_values1,
            lookup_indexes2,
            lookup_values2,
            unused(),
            unused(),
            unused(),
            unused(),
            unused(),
            unused(),
            unused(),
            unused(),
        ]
    };

    TestFramework::run_test_lookups(gates, witness, &[], lookup_tables);
}

#[test]
fn lookup_gate_proving_works() {
    setup_lookup_proof(true, 500, vec![256])
}

#[test]
#[should_panic]
fn lookup_gate_rejects_bad_lookups() {
    setup_lookup_proof(false, 500, vec![256])
}

#[test]
fn lookup_gate_proving_works_multiple_tables() {
    setup_lookup_proof(true, 500, vec![100, 50, 50, 2, 2])
}

#[test]
#[should_panic]
fn lookup_gate_rejects_bad_lookups_multiple_tables() {
    setup_lookup_proof(false, 500, vec![100, 50, 50, 2, 2])
}
