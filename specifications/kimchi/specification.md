---
weight: 3
bookFlatSection: false
title: "Kimchi"
summary: "This document specifies the kimchi variant of PLONK."
---

# Kimchi

**THIS IS WORK-IN-PROGRESS**

This document specifies the kimchi variant of PLONK.

## Overview

The document follows the following structure:

TODO: simply create a ToC no?

1. **Setup**. A one-time setup for the proof system.
2. **Per-circuit setup**. A one-time setup for each circuit that are used in the proof system.
3. **Proof creation**. How to create a proof.
4. **Proof verification**. How to verify a proof.

## Dependencies

### Polynomial Commitments

Refer to the [specification on polynomial commitments](). We make use of the following functions from that specification:

- `PolyCom.non_hiding_commit(poly) -> PolyCom::NonHidingCommitment`
- `PolyCom.commit(poly) -> PolyCom::HidingCommitment`
- `PolyCom.evaluation_proof(poly, commitment, point) -> EvaluationProof`
- `PolyCom.verify(commitment, point, evaluation, evaluation_proof) -> bool`

### Poseidon hash function

Refer to the [specification on Poseidon](). We make use of the following functions from that specification:

- `Poseidon.init(params) -> FqSponge`
- `Poseidon.update(field_elem)`
- `Poseidon.finalize() -> FieldElem`

specify the following functions on top:

- `Poseidon.produce_challenge()` (TODO: uses the endomorphism)
- `Poseidon.to_fr_sponge() -> state_of_fq_sponge_before_eval, FrSponge`

### Pasta

Kimchi is made to work on cycles of curves, so the protocol switch between two fields Fq and Fr, where Fq represents the base field and Fr represents the scalar field.

## Constraints

### Permutation

{sections.permutation}

### Lookup

{sections.lookup}

### Gates

#### Generic Gate

{sections.generic}

#### Poseidon

{sections.poseidon}

#### chacha 

{sections.chacha}

#### complete_add 

{sections.complete_add}

#### endomul_scalar 

{sections.endomul_scalar}

#### endosclmul 

{sections.endosclmul}

#### poseidon 

{sections.poseidon}

#### varbasemul 

{sections.varbasemul}

## constraint system creation (circuit creation)

{sections.constraint_system}

## prover and verifier index creation

{sections.indexes}

## proof data structure

TKTK

## proof creation

{sections.prover}

## proof verification

{sections.verifier}
