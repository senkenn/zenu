use crate::{
    dim::DimTrait,
    matrix::{MatrixBase, OwnedMatrix},
    num::Num,
};

pub trait Ones: MatrixBase {
    fn ones<I: Into<Self::Dim>>(dim: I) -> Self;
}

impl<D, T, OM> Ones for OM
where
    D: DimTrait,
    T: Num,
    OM: OwnedMatrix + MatrixBase<Dim = D, Item = T>,
{
    fn ones<I: Into<D>>(dim: I) -> Self {
        let dim = dim.into();
        let mut vec = vec![];
        for _ in 0..dim.num_elm() {
            vec.push(T::one());
        }
        Self::from_vec(vec, dim)
    }
}
