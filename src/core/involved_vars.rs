use crate::Literal;
use std::ops::{BitAndAssign, BitOrAssign, BitXorAssign};

type IntType = u32;

const N_BITS: usize = IntType::BITS as usize;

const MASKS: [IntType; N_BITS] = {
    let mut masks = [1; N_BITS];
    let mut i = 0;
    while i < N_BITS {
        masks[i] <<= i;
        i += 1;
    }
    masks
};

/// A type dedicated to the registration of the variables involved at some points.
#[derive(Clone, Debug)]
pub(crate) struct InvolvedVars {
    n_vars: usize,
    data: Vec<IntType>,
}

impl InvolvedVars {
    pub fn new(n_vars: usize) -> Self {
        Self {
            n_vars,
            data: vec![0; n_vars.div_ceil(N_BITS)],
        }
    }

    pub fn set_literal(&mut self, l: Literal) {
        let index = Literal::var_index(&l);
        let div = index / N_BITS;
        let rem = index % N_BITS;
        self.data[div] |= MASKS[rem];
    }

    pub fn is_set(&self, l: Literal) -> bool {
        let index = Literal::var_index(&l);
        let div = index / N_BITS;
        let rem = index % N_BITS;
        self.data[div] & MASKS[rem] > 0
    }

    pub fn any(&self) -> bool {
        self.data.iter().any(|i| *i > 0)
    }

    pub fn set_literals(&mut self, literals: &[Literal]) {
        literals.iter().for_each(|l| self.set_literal(*l));
    }

    pub fn iter_missing_literals(&self) -> impl Iterator<Item = Literal> + '_ {
        self.data
            .iter()
            .enumerate()
            .filter_map(|(offset, i)| {
                if *i == IntType::MAX {
                    None
                } else {
                    let it = BitToLitIterator::new(self.n_vars, offset * N_BITS, !*i);
                    Some(it.into_iter().map(|l| l.flip()))
                }
            })
            .flatten()
    }

    pub fn iter_xor_neg_literals<'a>(
        set0: &'a InvolvedVars,
        set1: &'a InvolvedVars,
    ) -> impl Iterator<Item = Literal> + 'a {
        set0.data
            .iter()
            .zip(set1.data.iter())
            .enumerate()
            .filter_map(|(offset, (int0, int1))| {
                let xor = int0 ^ int1;
                if xor == 0 {
                    None
                } else {
                    let it = BitToLitIterator::new(set0.n_vars, offset * N_BITS, xor);
                    Some(it.into_iter().map(|l| l.flip()))
                }
            })
            .flatten()
    }
}

macro_rules! decl_bit_assign {
    ($trait_name:ident, $eff_type:ty, $fn_name:ident) => {
        impl $trait_name<$eff_type> for InvolvedVars {
            fn $fn_name(&mut self, rhs: $eff_type) {
                self.data
                    .iter_mut()
                    .zip(rhs.data.iter())
                    .for_each(|(s, o)| s.$fn_name(o));
            }
        }
    };
}

macro_rules! decl_all_bit_assign {
    ($trait_name:ident, $fn_name:ident) => {
        decl_bit_assign!($trait_name, InvolvedVars, $fn_name);
        decl_bit_assign!($trait_name, &InvolvedVars, $fn_name);
        decl_bit_assign!($trait_name, &mut InvolvedVars, $fn_name);
    };
}
decl_all_bit_assign!(BitAndAssign, bitand_assign);
decl_all_bit_assign!(BitOrAssign, bitor_assign);
decl_all_bit_assign!(BitXorAssign, bitxor_assign);

struct BitToLitIterator {
    n_vars: IntType,
    offset: IntType,
    data: IntType,
}

impl BitToLitIterator {
    #[allow(clippy::cast_possible_truncation)]
    fn new(n_vars: usize, offset: usize, data: IntType) -> Self {
        Self {
            n_vars: n_vars as IntType,
            offset: offset as IntType,
            data,
        }
    }
}

impl Iterator for BitToLitIterator {
    type Item = Literal;

    #[allow(clippy::cast_possible_truncation)]
    fn next(&mut self) -> Option<Self::Item> {
        let trailing = self.data.trailing_zeros();
        if trailing == N_BITS as u32 || trailing + self.offset >= self.n_vars {
            None
        } else {
            self.data ^= MASKS[trailing as usize];
            Some(Literal::from((self.offset + trailing + 1) as isize))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_missing_literals_all_missing() {
        let involved = InvolvedVars::new(200);
        assert_eq!(
            (1..=200).map(|i| Literal::from(-i)).collect::<Vec<_>>(),
            involved.iter_missing_literals().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn test_iter_missing_literals_none_missing() {
        let mut involved = InvolvedVars::new(1);
        involved.set_literal(Literal::from(1));
        assert_eq!(
            vec![] as Vec<Literal>,
            involved.iter_missing_literals().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn test_bit_to_lit_it_first_bit() {
        let bits: IntType = 1;
        let it = BitToLitIterator::new(N_BITS, 0, bits);
        assert_eq!(it.collect::<Vec<_>>(), vec![Literal::from(1)]);
    }

    #[test]
    fn test_bit_to_lit_it_last_bit() {
        let bits: IntType = 1 << (N_BITS - 1);
        let it = BitToLitIterator::new(N_BITS, 0, bits);
        assert_eq!(it.collect::<Vec<_>>(), vec![Literal::from(N_BITS as isize)]);
    }
}
