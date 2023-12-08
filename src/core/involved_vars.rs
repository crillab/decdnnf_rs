use crate::Literal;
use bitvec::{bitvec, vec::BitVec};

#[derive(Clone)]
pub(crate) struct InvolvedVars(BitVec);

impl InvolvedVars {
    pub fn empty() -> Self {
        Self(BitVec::default())
    }

    pub fn new(n_vars: usize) -> Self {
        Self(bitvec![0; n_vars])
    }

    pub fn and_assign(&mut self, other: &InvolvedVars) {
        self.0 &= &other.0;
    }

    pub fn or_assign(&mut self, other: &InvolvedVars) {
        self.0 |= &other.0;
    }

    pub fn union(v: Vec<InvolvedVars>) -> Self {
        v.into_iter()
            .reduce(|mut acc, x| {
                acc.0 |= x.0;
                acc
            })
            .expect("cannot build union of 0 sets")
    }

    pub fn set_literal(&mut self, l: Literal) {
        self.0.set(l.var_index(), true);
    }

    pub fn set_literals(&mut self, literals: &[Literal]) {
        literals.iter().for_each(|l| self.set_literal(*l));
    }

    pub fn count_ones(&self) -> usize {
        self.0.count_ones()
    }

    pub fn count_zeros(&self) -> usize {
        self.0.count_zeros()
    }

    pub fn any(&self) -> bool {
        self.0.any()
    }
}
