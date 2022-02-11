//! This source file implements Plonk Protocol Index primitive.

use crate::alphas::{self, ConstraintType};
use crate::circuits::{
    constraints::{zk_polynomial, zk_w3, ConstraintSystem, LookupConstraintSystem},
    expr::{Column, ConstantExpr, Expr, Linearization, PolishToken},
    gate::{GateType, LookupsUsed},
    polynomials::{chacha, complete_add, endomul_scalar, endosclmul, lookup, poseidon, varbasemul},
    wires::*,
};
use ark_ec::AffineCurve;
use ark_ff::{FftField, PrimeField, SquareRootField};
use ark_poly::{univariate::DensePolynomial, Radix2EvaluationDomain as D};
use array_init::array_init;
use commitment_dlog::{
    commitment::{CommitmentCurve, PolyComm},
    srs::SRS,
    CommitmentField,
};
use oracle::poseidon::ArithmeticSpongeParams;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::serde_as;
use std::io::SeekFrom::Start;
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Seek},
    path::Path,
    sync::Arc,
};

//
// handy aliases
//

type Fr<G> = <G as AffineCurve>::ScalarField;
type Fq<G> = <G as AffineCurve>::BaseField;

//
// data structures
//

/// The index common to both the prover and verifier
// TODO: rename as ProverIndex
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Index<G: CommitmentCurve>
where
    G::ScalarField: CommitmentField,
{
    /// constraints system polynomials
    #[serde(bound = "ConstraintSystem<Fr<G>>: Serialize + DeserializeOwned")]
    pub cs: ConstraintSystem<Fr<G>>,

    /// The symbolic linearization of our circuit, which can compile to concrete types once certain values are learned in the protocol.
    #[serde(skip)]
    pub linearization: Linearization<Vec<PolishToken<Fr<G>>>>,

    /// The mapping between powers of alpha and constraints
    pub powers_of_alpha: alphas::Builder,

    /// polynomial commitment keys
    #[serde(skip)]
    pub srs: Arc<SRS<G>>,

    /// maximal size of polynomial section
    pub max_poly_size: usize,

    /// maximal size of the quotient polynomial according to the supported constraints
    pub max_quot_size: usize,

    /// random oracle argument parameters
    #[serde(skip)]
    pub fq_sponge_params: ArithmeticSpongeParams<Fq<G>>,
}

/// The verifier index

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct LookupVerifierIndex<G: CommitmentCurve> {
    pub lookup_used: LookupsUsed,
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub lookup_tables: Vec<Vec<PolyComm<G>>>,
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub lookup_selectors: Vec<PolyComm<G>>,
}

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct VerifierIndex<G: CommitmentCurve> {
    /// evaluation domain
    #[serde_as(as = "o1_utils::serialization::SerdeAs")]
    pub domain: D<Fr<G>>,
    /// maximal size of polynomial section
    pub max_poly_size: usize,
    /// maximal size of the quotient polynomial according to the supported constraints
    pub max_quot_size: usize,
    /// The mapping between powers of alpha and constraints
    pub powers_of_alpha: alphas::Builder,
    /// polynomial commitment keys
    #[serde(skip)]
    pub srs: Arc<SRS<G>>,

    // index polynomial commitments
    /// permutation commitment array
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub sigma_comm: [PolyComm<G>; PERMUTS],
    /// coefficient commitment array
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub coefficients_comm: [PolyComm<G>; COLUMNS],
    /// coefficient commitment array
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub generic_comm: PolyComm<G>,

    // poseidon polynomial commitments
    /// poseidon constraint selector polynomial commitment
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub psm_comm: PolyComm<G>,

    // ECC arithmetic polynomial commitments
    /// EC addition selector polynomial commitment
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub complete_add_comm: PolyComm<G>,
    /// EC variable base scalar multiplication selector polynomial commitment
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub mul_comm: PolyComm<G>,
    /// endoscalar multiplication selector polynomial commitment
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub emul_comm: PolyComm<G>,
    /// endoscalar multiplication scalar computation selector polynomial commitment
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub endomul_scalar_comm: PolyComm<G>,

    /// Chacha polynomial commitments
    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub chacha_comm: Option<[PolyComm<G>; 4]>,

    /// wire coordinate shifts
    #[serde_as(as = "[o1_utils::serialization::SerdeAs; PERMUTS]")]
    pub shift: [Fr<G>; PERMUTS],
    /// zero-knowledge polynomial
    #[serde(skip)]
    pub zkpm: DensePolynomial<Fr<G>>,
    // TODO(mimoo): isn't this redundant with domain.d1.group_gen ?
    /// domain offset for zero-knowledge
    #[serde(skip)]
    pub w: Fr<G>,
    /// endoscalar coefficient
    #[serde(skip)]
    pub endo: Fr<G>,

    #[serde(bound = "PolyComm<G>: Serialize + DeserializeOwned")]
    pub lookup_index: Option<LookupVerifierIndex<G>>,

    #[serde(skip)]
    pub linearization: Linearization<Vec<PolishToken<Fr<G>>>>,

    // random oracle argument parameters
    #[serde(skip)]
    pub fr_sponge_params: ArithmeticSpongeParams<Fr<G>>,
    #[serde(skip)]
    pub fq_sponge_params: ArithmeticSpongeParams<Fq<G>>,
}

//
// logic
//

/// construct the circuit constraint in expression form.
pub fn constraints_expr<F: FftField + SquareRootField>(
    domain: D<F>,
    chacha: bool,
    lookup_constraint_system: &Option<LookupConstraintSystem<F>>,
) -> (Expr<ConstantExpr<F>>, alphas::Builder) {
    // register powers of alpha so that we don't reuse them across mutually inclusive constraints
    let mut powers_of_alpha = alphas::Builder::default();

    // gates
    let alphas = powers_of_alpha.register(ConstraintType::Gate, 21);

    let mut expr = poseidon::constraint(alphas.clone().take(15));
    expr += varbasemul::constraint(alphas.clone().take(21));
    expr += complete_add::constraint(alphas.clone().take(7));
    expr += endosclmul::constraint(alphas.clone().take(11));
    expr += endomul_scalar::constraint(alphas.clone().take(11));

    // chacha
    if chacha {
        expr += chacha::constraint_chacha0(alphas.clone().take(5));
        expr += chacha::constraint_chacha1(alphas.clone().take(5));
        expr += chacha::constraint_chacha2(alphas.clone().take(5));
        expr += chacha::constraint_chacha_final(alphas.take(9))
    }

    // permutation
    let _alphas = powers_of_alpha.register(ConstraintType::Permutation, 3);

    // lookup
    if let Some(lcs) = lookup_constraint_system.as_ref() {
        let alphas = powers_of_alpha.register(ConstraintType::Lookup, 7);
        let constraints = lookup::constraints(&lcs.dummy_lookup_values[0], domain);
        let combined = Expr::combine_constraints(alphas, constraints);
        expr += combined;
    }

    // return the expression
    (expr, powers_of_alpha)
}

pub fn linearization_columns<F: FftField + SquareRootField>(
    lookup_constraint_system: &Option<LookupConstraintSystem<F>>,
) -> std::collections::HashSet<Column> {
    let mut h = std::collections::HashSet::new();
    use Column::*;
    for i in 0..COLUMNS {
        h.insert(Witness(i));
    }
    match lookup_constraint_system.as_ref() {
        None => (),
        Some(lcs) => {
            for i in 0..(lcs.max_lookups_per_row + 1) {
                h.insert(LookupSorted(i));
            }
        }
    }
    h.insert(Z);
    h.insert(LookupAggreg);
    h.insert(LookupTable);
    h.insert(Index(GateType::Poseidon));
    h.insert(Index(GateType::Generic));
    h
}

/// Returns a linearized expression in polish notation
pub fn expr_linearization<F: FftField + SquareRootField>(
    domain: D<F>,
    chacha: bool,
    lookup_constraint_system: &Option<LookupConstraintSystem<F>>,
) -> (Linearization<Vec<PolishToken<F>>>, alphas::Builder) {
    let evaluated_cols = linearization_columns::<F>(lookup_constraint_system);

    let (expr, powers_of_alpha) = constraints_expr(domain, chacha, lookup_constraint_system);

    let linearization = expr
        .linearize(evaluated_cols)
        .unwrap()
        .map(|e| e.to_polish());

    (linearization, powers_of_alpha)
}

//
// methods to create indexes
//

impl<'a, G: CommitmentCurve> Index<G>
where
    G::BaseField: PrimeField,
    G::ScalarField: CommitmentField,
{
    //~
    //~ ## Verifier Index
    //~
    //~ The verifier index is a structure that contains all the information needed to verify a proof.
    //~ You can create the verifier index from the prover index, by commiting to a number of polynomials in advance.
    //~

    pub fn verifier_index(&self) -> VerifierIndex<G> {
        let domain = self.cs.domain.d1;
        let lookup_index = {
            self.cs
                .lookup_constraint_system
                .as_ref()
                .map(|cs| LookupVerifierIndex {
                    lookup_used: cs.lookup_used,
                    lookup_selectors: cs
                        .lookup_selectors
                        .iter()
                        .map(|e| self.srs.commit_evaluations_non_hiding(domain, e, None))
                        .collect(),
                    lookup_tables: cs
                        .lookup_tables8
                        .iter()
                        .map(|v| {
                            v.iter()
                                .map(|e| self.srs.commit_evaluations_non_hiding(domain, e, None))
                                .collect()
                        })
                        .collect(),
                })
        };
        // TODO: Switch to commit_evaluations for all index polys
        VerifierIndex {
            domain,
            max_poly_size: self.max_poly_size,
            max_quot_size: self.max_quot_size,
            powers_of_alpha: self.powers_of_alpha.clone(),
            srs: Arc::clone(&self.srs),

            sigma_comm: array_init(|i| self.srs.commit_non_hiding(&self.cs.sigmam[i], None)),
            coefficients_comm: array_init(|i| {
                self.srs
                    .commit_evaluations_non_hiding(domain, &self.cs.coefficients8[i], None)
            }),
            generic_comm: self.srs.commit_non_hiding(&self.cs.genericm, None),

            psm_comm: self.srs.commit_non_hiding(&self.cs.psm, None),

            complete_add_comm: self.srs.commit_evaluations_non_hiding(
                domain,
                &self.cs.complete_addl4,
                None,
            ),
            mul_comm: self
                .srs
                .commit_evaluations_non_hiding(domain, &self.cs.mull8, None),
            emul_comm: self
                .srs
                .commit_evaluations_non_hiding(domain, &self.cs.emull, None),

            endomul_scalar_comm: self.srs.commit_evaluations_non_hiding(
                domain,
                &self.cs.endomul_scalar8,
                None,
            ),

            chacha_comm: self.cs.chacha8.as_ref().map(|c| {
                array_init(|i| self.srs.commit_evaluations_non_hiding(domain, &c[i], None))
            }),

            shift: self.cs.shift,
            zkpm: self.cs.zkpm.clone(),
            w: zk_w3(self.cs.domain.d1),
            endo: self.cs.endo,
            lookup_index,
            linearization: self.linearization.clone(),

            fr_sponge_params: self.cs.fr_sponge_params.clone(),
            fq_sponge_params: self.fq_sponge_params.clone(),
        }
    }

    //~
    //~ ## Prover Index
    //~
    //~ The prover index is a structure that contains all the information needed to
    //~ generate the proof.
    //~

    /// this function compiles the index from constraints
    pub fn create(
        mut cs: ConstraintSystem<Fr<G>>,
        fq_sponge_params: ArithmeticSpongeParams<Fq<G>>,
        endo_q: Fr<G>,
        srs: Arc<SRS<G>>,
    ) -> Self {
        let max_poly_size = srs.g.len();

        if cs.public > 0 {
            // TODO: why do this check only if there's public input?
            assert!(
                max_poly_size >= cs.domain.d1.size as usize,
                "polynomial segment size has to be not smaller that that of the circuit!"
            );
        }

        //~ 1. set the endomorphism value to endo_q

        cs.endo = endo_q; // TODO: this seems unrelated to the constraint system, store in index instead

        //~ 2. do the lookup stuff

        //
        // Lookup
        //

        let (linearization, powers_of_alpha) = expr_linearization(
            cs.domain.d1,
            cs.chacha8.is_some(),
            &cs.lookup_constraint_system,
        );

        let max_quot_size = PERMUTS * cs.domain.d1.size as usize;

        //~ 4. set the max quotient size to the number of IO registers times the domain size

        Index {
            cs,
            linearization,
            powers_of_alpha,
            srs,
            max_poly_size,
            max_quot_size,
            fq_sponge_params,
        }
    }
}

//
// (de)serialization methods
//

impl<G> VerifierIndex<G>
where
    G: CommitmentCurve,
{
    /// Deserializes a [VerifierIndex] from a file, given a pointer to an SRS and an optional offset in the file.
    pub fn from_file(
        srs: Arc<SRS<G>>,
        path: &Path,
        offset: Option<u64>,
        // TODO: we shouldn't have to pass these
        endo: G::ScalarField,
        fq_sponge_params: ArithmeticSpongeParams<Fq<G>>,
        fr_sponge_params: ArithmeticSpongeParams<Fr<G>>,
    ) -> Result<Self, String> {
        // open file
        let file = File::open(path).map_err(|e| e.to_string())?;

        // offset
        let mut reader = BufReader::new(file);
        if let Some(offset) = offset {
            reader.seek(Start(offset)).map_err(|e| e.to_string())?;
        }

        // deserialize
        let mut verifier_index = Self::deserialize(&mut rmp_serde::Deserializer::new(reader))
            .map_err(|e| e.to_string())?;

        // fill in the rest
        verifier_index.srs = srs;
        verifier_index.endo = endo;
        verifier_index.fq_sponge_params = fq_sponge_params;
        verifier_index.fr_sponge_params = fr_sponge_params;
        verifier_index.w = zk_w3(verifier_index.domain);
        verifier_index.zkpm = zk_polynomial(verifier_index.domain);

        Ok(verifier_index)
    }

    /// Writes a [VerifierIndex] to a file, potentially appending it to the already-existing content (if append is set to true)
    // TODO: append should be a bool, not an option
    pub fn to_file(&self, path: &Path, append: Option<bool>) -> Result<(), String> {
        let append = append.unwrap_or(true);
        let file = OpenOptions::new()
            .append(append)
            .open(path)
            .map_err(|e| e.to_string())?;

        let writer = BufWriter::new(file);

        self.serialize(&mut rmp_serde::Serializer::new(writer))
            .map_err(|e| e.to_string())
    }
}
