use crate::{DirectAccessEngine, Literal, ModelCounter, OrderedDirectAccessEngine};
use rug::{rand::RandState, Integer};
use rustc_hash::FxHashMap;

type DirectAccessFnType<'a> = Box<dyn Fn(Integer) -> Vec<Option<Literal>> + 'a>;

/// A structure used to sample the models of a [`DecisionDNNF`](crate::DecisionDNNF).
///
/// Sampling on such a structure in uniform, i.e. each model has the same chance to be chosen and without repetitions.
///
/// After creating a sampler with the [`new`](Self::new) function, call [`compute_next_model`](Self::compute_next_model) until you receive [`None`].
/// In case there are less models than the number of expected samples, no errors ill be thrown and the function will return [`None`].
/// In case the expected number of samples has been returned, [`None`] will be returned.
///
/// Each call that is not [`None`] returns a model which is different from the ones previously yielded.
/// The algorithm takes time that is polynomial in the number of models and space that is polynomial in the size of the Decision-DNNF.
///
/// # Examples
///
/// ```
/// use decdnnf_rs::{DecisionDNNF, ModelCounter, ModelSampler};
/// use rug::Integer;
///
/// fn print_some_models(ddnnf: &DecisionDNNF) {
///     let model_counter = ModelCounter::new(&ddnnf, false);
///     let mut model_sampler = ModelSampler::new(&model_counter, Integer::from(4));
///     println!("printing 4 models (max.)");
///     while let Some(model) = model_sampler.compute_next_model() {
///         print!("v");
///         for opt_l in model {
///             if let Some(l) = opt_l {
///                 print!(" {}", isize::from(l));
///             }
///         }
///         println!(" 0");
///     }
/// }
/// # use decdnnf_rs::DecisionDNNFReader;
/// # print_some_models(&decdnnf_rs::D4Reader::default().read("t 1 0".as_bytes()).unwrap())
/// ```
pub struct ModelSampler<'a> {
    model_counter: &'a ModelCounter<'a>,
    direct_access_engine: Option<DirectAccessFnType<'a>>,
    rand: RandState<'static>,
    n_samples: Integer,
    counter: Integer,
    order: Option<Vec<Literal>>,
    swapped: FxHashMap<Integer, Integer>,
}

impl<'a> ModelSampler<'a> {
    /// Builds a new model sampler given a model counter and the number of expected samples.
    ///
    /// The formula for which models are sampled is the one involved in the model counter.
    /// The model counter must be initialized in order to consider full models.
    ///
    /// # Panics
    ///
    /// This function panics if the model counter is initialized to count partial models.
    pub fn new(model_counter: &'a ModelCounter<'a>, n_samples: Integer) -> Self {
        assert!(!model_counter.partial_models());
        let n_models = Integer::from(model_counter.global_count());
        let init_n_samples = Integer::min(n_models, n_samples);
        Self {
            model_counter,
            direct_access_engine: None,
            rand: RandState::new_mersenne_twister(),
            n_samples: init_n_samples,
            counter: Integer::ZERO,
            order: None,
            swapped: FxHashMap::default(),
        }
    }

    /// The models are now "ordered" using the lexicographic order.
    ///
    /// Setting an order allow the reproducibility for equal seeds and equivalent formulas.
    /// This function must be called before the first call to [`compute_next_model`](Self::compute_next_model).
    ///
    /// # Panics
    ///
    /// This function panics if it is called after [`compute_next_model`](Self::compute_next_model).
    pub fn set_lexicographic_order(&mut self) {
        let order = (1..=self.model_counter.ddnnf().n_vars())
            .map(|i| Literal::from(-isize::try_from(i).unwrap()))
            .collect::<Vec<_>>();
        self.set_order(order);
    }

    /// The models are now "ordered" using the provided order.
    ///
    /// See [`DirectAccessEngine`](crate::DirectAccessEngine) for orders.
    /// Setting an order allow the reproducibility for equal seeds and equivalent formulas.
    /// This function must be called before the first call to [`compute_next_model`](Self::compute_next_model).
    ///
    /// # Panics
    ///
    /// This function panics if it is called after [`compute_next_model`](Self::compute_next_model).
    pub fn set_order(&mut self, order: Vec<Literal>) {
        assert!(
            self.direct_access_engine.is_none(),
            "cannot set order after sampling has begun"
        );
        self.order = Some(order);
    }

    /// Sets the seed for the pseudo-random number generator.
    ///
    /// Setting a seed allow reproducibility when the same formula is involved.
    /// This function must be called before the first call to [`compute_next_model`](Self::compute_next_model).
    ///
    /// # Panics
    ///
    /// This function panics if it is called after [`compute_next_model`](Self::compute_next_model).
    pub fn set_seed(&mut self, seed: &Integer) {
        assert!(
            self.direct_access_engine.is_none(),
            "cannot set seed after sampling has begun"
        );
        self.rand.seed(seed);
    }

    /// Returns the effective number of samples to be returned.
    ///
    /// It depends on both the number asked when the object is created and the number of models the formula has.
    #[must_use]
    pub fn n_samples_remaining(&self) -> Integer {
        Integer::from(&self.n_samples - &self.counter)
    }

    /// Computes the next sample.
    ///
    /// This function returns [`None`] if the number of returned samples already reached the number entered when the sampler was created or if all the models have been enumerated.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn compute_next_model(&mut self) -> Option<Vec<Option<Literal>>> {
        if self.direct_access_engine.is_none() {
            self.direct_access_engine = Some(self.create_direct_access_engine());
        }
        if self.counter == self.n_samples {
            return None;
        }
        let mut bound = Integer::from(self.model_counter.global_count() - &self.counter);
        self.counter += 1;
        let rand_index = Integer::from(bound.random_below_ref(&mut self.rand));
        bound -= 1;
        let last_value = self.swapped.get(&bound).unwrap_or(&bound).to_owned();
        let rand_value = self
            .swapped
            .insert(rand_index.clone(), last_value)
            .unwrap_or(rand_index);
        let model = self.direct_access_engine.as_ref().unwrap()(rand_value);
        Some(model)
    }

    fn create_direct_access_engine(&self) -> DirectAccessFnType<'a> {
        if let Some(o) = self.order.clone() {
            let engine = OrderedDirectAccessEngine::new(self.model_counter.ddnnf(), o).unwrap();
            Box::new(move |i| {
                engine
                    .model(i)
                    .unwrap()
                    .iter()
                    .map(|l| Some(*l))
                    .collect::<Vec<_>>()
            })
        } else {
            let engine = DirectAccessEngine::new(self.model_counter);
            Box::new(move |i| engine.model(i).unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{D4Reader, DecisionDNNFReader};

    const EXPECTED_TRIVIAL_2VARS: &[&[isize]] = &[&[-1, -2], &[-1, 2], &[1, -2], &[1, 2]];

    fn assert_samples_in(sampler: &mut ModelSampler<'_>, n_expected: usize, expected: &[&[isize]]) {
        let mut expected: Vec<Vec<Option<Literal>>> = expected
            .iter()
            .map(|m| m.iter().map(|i| Some(Literal::from(*i))).collect())
            .collect();
        let actual = sample_all(sampler);
        for m in &actual {
            let pos = expected.iter().position(|m0| m == m0).unwrap();
            expected.swap_remove(pos);
        }
        assert_eq!(n_expected, actual.len());
    }

    fn sample_all(sampler: &mut ModelSampler<'_>) -> Vec<Vec<Option<Literal>>> {
        let mut s = Vec::new();
        while let Some(m) = sampler.compute_next_model() {
            s.push(m);
        }
        s
    }

    #[test]
    fn test_sample_some() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let counter = ModelCounter::new(&ddnnf, false);
        let mut sampler = ModelSampler::new(&counter, Integer::from(2));
        assert_eq!(2, sampler.n_samples_remaining());
        assert_samples_in(&mut sampler, 2, EXPECTED_TRIVIAL_2VARS);
        assert_eq!(0, sampler.n_samples_remaining());
    }

    #[test]
    fn test_sample_all() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let counter = ModelCounter::new(&ddnnf, false);
        let mut sampler = ModelSampler::new(&counter, Integer::from(10));
        assert_eq!(4, sampler.n_samples_remaining());
        assert_samples_in(&mut sampler, 4, EXPECTED_TRIVIAL_2VARS);
        assert_eq!(0, sampler.n_samples_remaining());
    }

    #[test]
    fn test_sample_some_lexico() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let counter = ModelCounter::new(&ddnnf, false);
        let mut sampler = ModelSampler::new(&counter, Integer::from(2));
        sampler.set_lexicographic_order();
        assert_eq!(2, sampler.n_samples_remaining());
        assert_samples_in(&mut sampler, 2, EXPECTED_TRIVIAL_2VARS);
        assert_eq!(0, sampler.n_samples_remaining());
    }

    #[test]
    fn test_sample_all_lexico() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let counter = ModelCounter::new(&ddnnf, false);
        let mut sampler = ModelSampler::new(&counter, Integer::from(10));
        sampler.set_lexicographic_order();
        assert_eq!(4, sampler.n_samples_remaining());
        assert_samples_in(&mut sampler, 4, EXPECTED_TRIVIAL_2VARS);
        assert_eq!(0, sampler.n_samples_remaining());
    }

    #[test]
    #[should_panic(expected = "cannot set order after sampling has begun")]
    fn test_lexico_too_late() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let counter = ModelCounter::new(&ddnnf, false);
        let mut sampler = ModelSampler::new(&counter, Integer::from(10));
        let _ = sampler.compute_next_model();
        sampler.set_lexicographic_order();
    }

    #[test]
    fn test_seed_repr() {
        const MAX: usize = 8;
        let v0 = (0..MAX)
            .map(|i| {
                let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
                ddnnf.update_n_vars(2);
                let counter = ModelCounter::new(&ddnnf, false);
                let mut sampler = ModelSampler::new(&counter, Integer::from(10));
                sampler.set_seed(&Integer::from(i));
                sample_all(&mut sampler)
            })
            .collect::<Vec<_>>();
        let v1 = (0..MAX)
            .map(|i| {
                let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
                ddnnf.update_n_vars(2);
                let counter = ModelCounter::new(&ddnnf, false);
                let mut sampler = ModelSampler::new(&counter, Integer::from(10));
                sampler.set_seed(&Integer::from(i));
                sample_all(&mut sampler)
            })
            .collect::<Vec<_>>();
        for i in 0..MAX {
            assert_eq!(&v0[i], &v1[i]);
            assert!(v0.iter().any(|v| v != &v1[i]));
        }
    }

    #[test]
    #[should_panic(expected = "cannot set seed after sampling has begun")]
    fn test_seed_too_late() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let counter = ModelCounter::new(&ddnnf, false);
        let mut sampler = ModelSampler::new(&counter, Integer::from(10));
        let _ = sampler.compute_next_model();
        sampler.set_seed(&Integer::ZERO);
    }
}
