use crate::Literal;
use bitvec::{bitvec, vec::BitVec};

/// A type dedicated to the registration of the variables involved at some points.
/// Relies on bitsets.
#[derive(Clone, Debug)]
pub(crate) struct InvolvedVars(BitVec);

impl InvolvedVars {
    pub fn new(n_vars: usize) -> Self {
        Self(bitvec![0; n_vars])
    }

    pub fn and_assign(&mut self, other: &InvolvedVars) {
        self.0 &= &other.0;
    }

    pub fn or_assign(&mut self, other: &InvolvedVars) {
        self.0 |= &other.0;
    }

    pub fn xor_assign(&mut self, other: &InvolvedVars) {
        self.0 ^= &other.0;
    }

    pub fn set_literal(&mut self, l: Literal) {
        self.0.set(l.var_index(), true);
    }

    pub fn set_literals(&mut self, literals: &[Literal]) {
        literals.iter().for_each(|l| self.set_literal(*l));
    }

    pub fn is_set(&self, l: Literal) -> bool {
        *self.0.get(l.var_index()).unwrap()
    }

    pub fn iter_missing_literals(&self) -> impl Iterator<Item = Literal> + '_ {
        self.0
            .iter_zeros()
            .map(|i| Literal::from(-isize::try_from(i + 1).unwrap()))
    }

    pub fn iter_neg_literals(&self) -> impl Iterator<Item = Literal> + '_ {
        self.0
            .iter_ones()
            .map(|i| Literal::from(-isize::try_from(i + 1).unwrap()))
    }

    pub fn any(&self) -> bool {
        self.0.any()
    }
}
