use crate::Literal;
use bitvec::{bitvec, vec::BitVec};

/// A type dedicated to the registration of the variables involved at some points.
/// Relies on bitsets.
#[derive(Clone, Debug)]
pub(crate) struct InvolvedVars(BitVec);

impl InvolvedVars {
    pub fn empty() -> Self {
        Self(BitVec::default())
    }

    pub fn new(n_vars: usize) -> Self {
        Self(bitvec![0; n_vars])
    }

    pub fn new_all_set(n_vars: usize) -> Self {
        Self(bitvec![1; n_vars])
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

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn count_ones(&self) -> usize {
        self.0.count_ones()
    }

    pub fn count_zeros(&self) -> usize {
        self.0.count_zeros()
    }

    pub fn iter_missing_literals(&self) -> impl Iterator<Item = Literal> + '_ {
        self.0
            .iter_zeros()
            .map(|i| Literal::from(isize::try_from(i + 1).unwrap()))
    }

    pub fn iter_pos_literals(&self) -> impl Iterator<Item = Literal> + '_ {
        self.0
            .iter_ones()
            .map(|i| Literal::from(isize::try_from(i + 1).unwrap()))
    }

    pub fn any(&self) -> bool {
        self.0.any()
    }
}
