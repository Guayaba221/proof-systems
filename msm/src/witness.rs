use ark_ff::Zero;
use rayon::iter::{FromParallelIterator, IntoParallelIterator, ParallelIterator};
use std::ops::Index;

/// The witness columns used by a gate of the MSM circuits.
/// It is generic over the number of columns, N, and the type of the witness, T.
/// It is parametrized by a type `T` which can be either:
/// - `Vec<G::ScalarField>` for the evaluations
/// - `PolyComm<G>` for the commitments
/// It can be used to represent the different subcircuits used by the project.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Witness<const N: usize, T> {
    /// A witness row is represented by an array of N witness columns
    /// When T is a vector, then the witness describes the rows of the circuit.
    pub cols: Box<[T; N]>,
}

impl<const N: usize, T: Zero + Clone> Default for Witness<N, T> {
    fn default() -> Self {
        Witness {
            cols: Box::new(std::array::from_fn(|_| T::zero())),
        }
    }
}

impl<const N: usize, T> Index<usize> for Witness<N, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.cols[index]
    }
}

impl<const N: usize, T> Witness<N, T> {
    pub fn len(&self) -> usize {
        self.cols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cols.is_empty()
    }
}

impl<const N: usize, T: Zero + Clone> Witness<N, Vec<T>> {
    pub fn zero_vec(domain_size: usize) -> Self {
        Witness {
            // Ideally the vector should be of domain size, but
            // one-element vector should be a reasonable default too.
            cols: Box::new(std::array::from_fn(|_| vec![T::zero(); domain_size])),
        }
    }

    pub fn to_pub_columns<const NPUB: usize>(&self) -> Witness<NPUB, Vec<T>> {
        let mut newcols: [Vec<T>; NPUB] = std::array::from_fn(|_| vec![]);
        for (i, vec) in self.cols[0..NPUB].iter().enumerate() {
            newcols[i] = vec.clone();
        }
        Witness {
            cols: Box::new(newcols),
        }
    }
}

// IMPLEMENTATION OF ITERATORS FOR THE WITNESS STRUCTURE

impl<'lt, const N: usize, G> IntoIterator for &'lt Witness<N, G> {
    type Item = &'lt G;
    type IntoIter = std::vec::IntoIter<&'lt G>;

    fn into_iter(self) -> Self::IntoIter {
        let mut iter_contents = Vec::with_capacity(N);
        iter_contents.extend(&*self.cols);
        iter_contents.into_iter()
    }
}

impl<const N: usize, F: Clone> IntoIterator for Witness<N, F> {
    type Item = F;
    type IntoIter = std::vec::IntoIter<F>;

    /// Iterate over the columns in the circuit.
    fn into_iter(self) -> Self::IntoIter {
        let mut iter_contents = Vec::with_capacity(N);
        iter_contents.extend(*self.cols);
        iter_contents.into_iter()
    }
}

impl<const N: usize, G> IntoParallelIterator for Witness<N, G>
where
    Vec<G>: IntoParallelIterator,
{
    type Iter = <Vec<G> as IntoParallelIterator>::Iter;
    type Item = <Vec<G> as IntoParallelIterator>::Item;

    /// Iterate over the columns in the circuit, in parallel.
    fn into_par_iter(self) -> Self::Iter {
        let mut iter_contents = Vec::with_capacity(N);
        iter_contents.extend(*self.cols);
        iter_contents.into_par_iter()
    }
}

impl<const N: usize, G: Send + std::fmt::Debug> FromParallelIterator<G> for Witness<N, G> {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = G>,
    {
        let mut iter_contents = par_iter.into_par_iter().collect::<Vec<_>>();
        let cols = iter_contents
            .drain(..N)
            .collect::<Vec<G>>()
            .try_into()
            .unwrap();
        Witness { cols }
    }
}

impl<'data, const N: usize, G> IntoParallelIterator for &'data Witness<N, G>
where
    Vec<&'data G>: IntoParallelIterator,
{
    type Iter = <Vec<&'data G> as IntoParallelIterator>::Iter;
    type Item = <Vec<&'data G> as IntoParallelIterator>::Item;

    fn into_par_iter(self) -> Self::Iter {
        let mut iter_contents = Vec::with_capacity(N);
        iter_contents.extend(&*self.cols);
        iter_contents.into_par_iter()
    }
}

impl<'data, const N: usize, G> IntoParallelIterator for &'data mut Witness<N, G>
where
    Vec<&'data mut G>: IntoParallelIterator,
{
    type Iter = <Vec<&'data mut G> as IntoParallelIterator>::Iter;
    type Item = <Vec<&'data mut G> as IntoParallelIterator>::Item;

    fn into_par_iter(self) -> Self::Iter {
        let mut iter_contents = Vec::with_capacity(N);
        iter_contents.extend(&mut *self.cols);
        iter_contents.into_par_iter()
    }
}
