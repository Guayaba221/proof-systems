use ark_ec::AffineCurve;
use ark_ff::{Field, PrimeField};
use commitment_dlog::{commitment::CommitmentCurve, srs::endos};
use mina_curves::pasta::{pallas::Pallas as PallasAffine, vesta::Vesta as VestaAffine, Fp, Fq};
use oracle::poseidon::ArithmeticSpongeParams;

#[derive(Clone)]
pub struct Constants<F: Field> {
    pub poseidon: ArithmeticSpongeParams<F>,
    pub endo: F,
    pub base: (F, F),
}

pub trait KimchiParams: PrimeField {
    fn constants() -> Constants<Self>;
}

impl KimchiParams for Fp {
    fn constants() -> Constants<Fp> {
        let (endo_q, _endo_r) = endos::<PallasAffine>();
        let base = PallasAffine::prime_subgroup_generator()
            .to_coordinates()
            .unwrap();
        Constants {
            poseidon: oracle::pasta::fp_kimchi::params(),
            endo: endo_q,
            base,
        }
    }
}

impl KimchiParams for Fq {
    fn constants() -> Constants<Fq> {
        let (endo_q, _endo_r) = endos::<VestaAffine>();
        let base = VestaAffine::prime_subgroup_generator()
            .to_coordinates()
            .unwrap();
        Constants {
            poseidon: oracle::pasta::fq_kimchi::params(),
            endo: endo_q,
            base,
        }
    }
}
