use ark_ff::Field;
use kimchi::circuits::{
    expr::{ConstantExpr, ConstantTerm, Expr, ExprInner, Variable},
    gate::CurrOrNext,
};

use crate::{columns::Column, LIMBS_NUM};

use super::interpreter::InterpreterEnv;

pub struct Env<Fp> {
    pub constraints: Vec<Expr<ConstantExpr<Fp>, Column>>,
}

impl<F: Field> InterpreterEnv for Env<F> {
    type Position = Column;

    type Variable = Expr<ConstantExpr<F>, Column>;

    fn add_constraint(&mut self, cst: Self::Variable) {
        self.constraints.push(cst)
    }

    fn copy(&mut self, x: &Self::Variable, position: Self::Position) -> Self::Variable {
        let y = Expr::Atom(ExprInner::Cell(Variable {
            col: position,
            row: CurrOrNext::Curr,
        }));
        self.constraints.push(y.clone() - x.clone());
        y
    }

    fn get_column_for_kimchi_limb(j: usize) -> Self::Position {
        assert!(j < 3);
        Column::X(j)
    }

    fn get_column_for_intermediate_limb(j: usize) -> Self::Position {
        assert!(j < 19);
        Column::X(3 + LIMBS_NUM + j)
    }

    fn get_column_for_msm_limb(j: usize) -> Self::Position {
        assert!(j < LIMBS_NUM);
        Column::X(3 + j)
    }

    fn constant(value: u128) -> Self::Variable {
        let value = F::from(value);
        let cst_expr_inner = ConstantExpr::from(ConstantTerm::Literal(value));
        Expr::Atom(ExprInner::Constant(cst_expr_inner))
    }

    /// Extract the bits from the variable `x` between `highest_bit` and `lowest_bit`, and store
    /// the result in `position`.
    /// `lowest_bit` becomes the least-significant bit of the resulting value.
    /// The value `x` is expected to be encoded in big-endian
    fn bitmask_be(
        &mut self,
        _x: &Self::Variable,
        _highest_bit: u32,
        _lowest_bit: u32,
        _position: Self::Position,
    ) -> Self::Variable {
        unimplemented!()
    }
}
