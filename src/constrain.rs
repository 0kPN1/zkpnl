use bulletproofs::r1cs::{ConstraintSystem, LinearCombination};

pub fn equal<CS: ConstraintSystem, Rhs: Into<LinearCombination>>(cs: &mut CS, lhs: LinearCombination, rhs: Rhs) {
    cs.constrain(lhs - rhs.into());
}