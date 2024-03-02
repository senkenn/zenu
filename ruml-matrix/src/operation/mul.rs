use crate::{
    blas::Blas,
    dim::{Dim1, Dim2, Dim3, Dim4, DimTrait},
    index::Index0D,
    matrix::{AsMutPtr, AsPtr, IndexAxisDyn, IndexAxisMutDyn, MatrixBase, ViewMutMatix},
    matrix_blas::gemm::gemm,
    matrix_impl::{matrix_into_dim, Matrix},
    memory::{View, ViewMut},
    memory_impl::{ViewMem, ViewMutMem},
    num::Num,
    operation::copy_from::CopyFrom,
};

fn mul_matrix_scalar<T, LM, SM, D>(self_: Matrix<SM, D>, lhs: Matrix<LM, D>, rhs: T)
where
    T: Num,
    SM: ViewMut<Item = T>,
    LM: View<Item = T>,
    D: DimTrait,
{
    assert_eq!(self_.shape(), lhs.shape());
    let mut self_ = self_.into_dyn_dim();
    let lhs = lhs.into_dyn_dim();
    CopyFrom::copy_from(&mut self_, &lhs);
    mul_assign_matrix_scalar(self_, rhs);
}

fn mul_assign_matrix_scalar<T, LM, D>(to: Matrix<LM, D>, other: T)
where
    T: Num,
    LM: ViewMut<Item = T>,
    D: DimTrait,
{
    // let mut to = to.into_dyn_dim();

    macro_rules! scal {
        ($dim:ty) => {{
            let mut to: Matrix<LM, $dim> = matrix_into_dim(to);
            if to.shape_stride().is_contiguous() {
                let num_dim = to.shape().len();
                LM::Blas::scal(
                    to.shape().num_elm(),
                    other,
                    to.as_mut_ptr(),
                    to.stride()[num_dim - 1],
                );
            } else {
                for idx in 0..to.shape()[0] {
                    let to_ = to.index_axis_mut_dyn(Index0D::new(idx));
                    mul_assign_matrix_scalar(to_, other);
                }
            }
        }};
    }

    match to.shape().len() {
        0 => {
            let mut to = to;
            LM::Blas::scal(1, other, to.as_mut_ptr(), 1);
        }
        1 => {
            let mut to = to;
            LM::Blas::scal(to.shape().num_elm(), other, to.as_mut_ptr(), to.stride()[0]);
        }
        2 => scal!(Dim2),
        3 => scal!(Dim3),
        4 => scal!(Dim4),
        _ => panic!("not implemented: this is bug. please report this bug."),
    };
}

fn mul_matrix_matrix<T, D1, D2, D3>(
    self_: Matrix<ViewMutMem<T>, D1>,
    lhs: Matrix<ViewMem<T>, D2>,
    rhs: Matrix<ViewMem<T>, D3>,
) where
    T: Num,
    D1: DimTrait,
    D2: DimTrait,
    D3: DimTrait,
{
    if lhs.shape().len() < rhs.shape().len() {
        mul_matrix_matrix(self_, rhs, lhs);
        return;
    }
    assert_eq!(self_.shape().slice(), lhs.shape().slice());
    let mut self_ = self_.into_dyn_dim();
    let lhs = lhs.into_dyn_dim();
    self_.copy_from(&lhs);
    dbg!(self_.shape());
    mul_assign_matrix_matrix(self_, rhs);
}

/// 入力の行列はsliceされていないものが必ず入力される
/// shape, strideに関するチェックは一切行いわない
fn mul_1d_1d_cpu<T: Num, D: DimTrait>(
    to: &mut Matrix<ViewMutMem<T>, D>,
    other: &Matrix<ViewMem<T>, D>,
) {
    let to_stride = to.stride();
    let other_stride = other.stride();
    let num_elm = to.shape().num_elm();
    let inner_stride_to = to_stride[to_stride.len() - 1];
    let inner_stride_other = other_stride[other_stride.len() - 1];
    let to_slice = to.as_mut_slice();
    let other_slice = other.as_slice();
    if inner_stride_to == 0 && inner_stride_other == 0 {
        for i in 0..num_elm {
            to_slice[i] *= other_slice[i];
        }
    } else {
        for i in 0..num_elm {
            to_slice[i * inner_stride_to] *= other_slice[i * inner_stride_other];
        }
    }
}

fn mul_assign_matrix_matrix<T, D1, D2>(to: Matrix<ViewMutMem<T>, D1>, other: Matrix<ViewMem<T>, D2>)
where
    T: Num,
    D1: DimTrait,
    D2: DimTrait,
{
    let mut to = to.into_dyn_dim();
    let other = other.into_dyn_dim();

    assert!(to.shape().is_include(&other.shape()));
    if to.shape().is_empty() {
        let ptr = to.as_mut_ptr();
        let other = other.as_ptr();
        unsafe {
            *ptr = *other * *ptr;
        }
        return;
    }
    if other.shape().is_empty() {
        let scalar = unsafe { *other.as_ptr() };
        mul_assign_matrix_scalar(to, scalar);
        return;
    }

    if to.shape().len() == 1 {
        let mut to: Matrix<ViewMutMem<T>, Dim1> = matrix_into_dim(to);
        let other: Matrix<ViewMem<T>, Dim1> = matrix_into_dim(other);
        mul_1d_1d_cpu(&mut to, &other);
    } else if to.shape() == other.shape() {
        macro_rules! same_dim {
            ($dim:ty) => {{
                let mut to: Matrix<ViewMutMem<T>, $dim> = matrix_into_dim(to);
                let other: Matrix<ViewMem<T>, $dim> = matrix_into_dim(other);

                for i in 0..to.shape()[0] {
                    let to_ = to.index_axis_mut_dyn(Index0D::new(i));
                    let other = other.index_axis_dyn(Index0D::new(i));
                    mul_assign_matrix_matrix(to_, other);
                }
            }};
        }
        match to.shape().len() {
            2 => same_dim!(Dim2),
            3 => same_dim!(Dim3),
            4 => same_dim!(Dim4),
            _ => panic!("not implemented: this is bug. please report this bug."),
        }
    } else {
        macro_rules! diff_dim {
            ($dim1:ty, $dim2:ty) => {{
                let mut to: Matrix<ViewMutMem<T>, $dim1> = matrix_into_dim(to);
                let other: Matrix<ViewMem<T>, $dim2> = matrix_into_dim(other);
                for i in 0..to.shape()[0] {
                    let to_ = to.index_axis_mut_dyn(Index0D::new(i));
                    mul_assign_matrix_matrix(to_, other.clone());
                }
            }};
        }
        match (to.shape().len(), other.shape().len()) {
            (2, 1) => diff_dim!(Dim2, Dim1),
            (3, 1) => diff_dim!(Dim3, Dim1),
            (4, 1) => diff_dim!(Dim4, Dim1),
            (3, 2) => diff_dim!(Dim3, Dim2),
            (4, 2) => diff_dim!(Dim4, Dim2),
            (4, 3) => diff_dim!(Dim4, Dim3),
            _ => panic!("not implemented: this is bug. please report this bug."),
        }
    }
}

pub trait MatrixMul<Lhs, Rhs>: ViewMutMatix {
    fn mul(self, lhs: Lhs, rhs: Rhs);
}

impl<T, RM, SM, D> MatrixMul<Matrix<RM, D>, T> for Matrix<SM, D>
where
    T: Num,
    RM: View<Item = T>,
    SM: ViewMut<Item = T>,
    D: DimTrait,
{
    fn mul(self, lhs: Matrix<RM, D>, rhs: T) {
        mul_matrix_scalar(self, lhs, rhs);
    }
}

impl<'a, 'b, 'c, T, DS, DR> MatrixMul<Matrix<ViewMem<'a, T>, DS>, Matrix<ViewMem<'b, T>, DR>>
    for Matrix<ViewMutMem<'c, T>, DS>
where
    T: Num,
    DS: DimTrait,
    DR: DimTrait,
{
    fn mul(self, lhs: Matrix<ViewMem<T>, DS>, rhs: Matrix<ViewMem<T>, DR>) {
        mul_matrix_matrix(self, lhs, rhs);
    }
}

pub trait Gemm<Rhs, Lhs>: ViewMutMatix {
    fn gemm(self, rhs: Rhs, lhs: Lhs);
}

impl<'a, 'b, 'c, T, D1, D2, D3> Gemm<Matrix<ViewMem<'a, T>, D1>, Matrix<ViewMem<'b, T>, D2>>
    for Matrix<ViewMutMem<'c, T>, D3>
where
    T: Num,
    D1: DimTrait,
    D2: DimTrait,
    D3: DimTrait,
{
    fn gemm(self, rhs: Matrix<ViewMem<T>, D1>, lhs: Matrix<ViewMem<T>, D2>) {
        assert_eq!(self.shape().len(), 2);
        assert_eq!(rhs.shape().len(), 2);
        assert_eq!(lhs.shape().len(), 2);
        let self_ = matrix_into_dim(self);
        let rhs = matrix_into_dim(rhs);
        let lhs = matrix_into_dim(lhs);
        gemm(rhs, lhs, self_, T::one(), T::zero());
    }
}

#[cfg(test)]
mod mul {
    use crate::{
        matrix::{IndexItem, MatrixSlice, OwnedMatrix, ToViewMatrix, ToViewMutMatrix},
        matrix_impl::{OwnedMatrix0D, OwnedMatrix1D, OwnedMatrix2D, OwnedMatrix4D, OwnedMatrixDyn},
        operation::{ones::Ones, zeros::Zeros},
        slice,
    };

    use super::MatrixMul;

    #[test]
    fn mul_1d_scalar() {
        let a = OwnedMatrix1D::from_vec(vec![1.0, 2.0, 3.0], [3]);
        let b = OwnedMatrix0D::from_vec(vec![2.0], []);
        let mut ans = OwnedMatrix1D::<f32>::zeros([3]);
        ans.to_view_mut().mul(a.to_view(), b.to_view());

        assert_eq!(ans.index_item([0]), 2.0);
        assert_eq!(ans.index_item([1]), 4.0);
        assert_eq!(ans.index_item([2]), 6.0);
    }

    #[test]
    fn scalar_1d() {
        let a = OwnedMatrix1D::from_vec(vec![1., 2., 3.], [3]);
        let mut ans = OwnedMatrix1D::<f32>::zeros([3]);
        ans.to_view_mut().mul(a.to_view(), 2.);

        assert_eq!(ans.index_item([0]), 2.);
        assert_eq!(ans.index_item([1]), 4.);
        assert_eq!(ans.index_item([2]), 6.);
    }

    #[test]
    fn sliced_scalar_1d() {
        let a = OwnedMatrix1D::from_vec(vec![1., 2., 3., 4.], [4]);
        let mut ans = OwnedMatrix1D::<f32>::zeros([2]);
        ans.to_view_mut().mul(a.to_view().slice(slice!(..;2)), 2.);

        assert_eq!(ans.index_item([0]), 2.);
        assert_eq!(ans.index_item([1]), 6.);
    }

    #[test]
    fn scalar_2d() {
        let a = OwnedMatrix2D::from_vec(vec![1., 2., 3., 4., 5., 6.], [2, 3]);
        let mut ans = OwnedMatrix2D::<f32>::zeros([2, 3]);
        ans.to_view_mut().mul(a.to_view(), 2.);

        assert_eq!(ans.index_item([0, 0]), 2.);
        assert_eq!(ans.index_item([0, 1]), 4.);
        assert_eq!(ans.index_item([0, 2]), 6.);
        assert_eq!(ans.index_item([1, 0]), 8.);
        assert_eq!(ans.index_item([1, 1]), 10.);
        assert_eq!(ans.index_item([1, 2]), 12.);
    }

    #[test]
    fn default_1d_1d() {
        let a = OwnedMatrix1D::from_vec(vec![1., 2., 3.], [3]);
        let b = OwnedMatrix1D::from_vec(vec![1., 2., 3.], [3]);
        let mut ans = OwnedMatrix1D::<f32>::zeros([3]);
        ans.to_view_mut().mul(a.to_view(), b.to_view());

        assert_eq!(ans.index_item([0]), 1.);
        assert_eq!(ans.index_item([1]), 4.);
        assert_eq!(ans.index_item([2]), 9.);
    }

    #[test]
    fn sliced_1d_1d() {
        let a = OwnedMatrix1D::from_vec(vec![1., 2., 3., 4.], [4]);
        let b = OwnedMatrix1D::from_vec(vec![1., 2., 3., 4.], [4]);
        let mut ans = OwnedMatrix1D::<f32>::zeros([2]);
        ans.to_view_mut().mul(
            a.to_view().slice(slice!(..;2)),
            b.to_view().slice(slice!(..;2)),
        );

        assert_eq!(ans.index_item([0]), 1.);
        assert_eq!(ans.index_item([1]), 9.);
    }

    #[test]
    fn default_2d_2d() {
        let a = OwnedMatrix2D::from_vec(vec![1., 2., 3., 4., 5., 6.], [2, 3]);
        let b = OwnedMatrix2D::from_vec(vec![1., 2., 3., 4., 5., 6.], [2, 3]);
        let mut ans = OwnedMatrix2D::<f32>::zeros([2, 3]);
        ans.to_view_mut().mul(a.to_view(), b.to_view());

        assert_eq!(ans.index_item([0, 0]), 1.);
        assert_eq!(ans.index_item([0, 1]), 4.);
        assert_eq!(ans.index_item([0, 2]), 9.);
        assert_eq!(ans.index_item([1, 0]), 16.);
        assert_eq!(ans.index_item([1, 1]), 25.);
        assert_eq!(ans.index_item([1, 2]), 36.);
    }

    #[test]
    fn sliced_4d_2d() {
        let mut a_vec = Vec::new();
        for i in 0..2 * 2 * 2 * 2 {
            a_vec.push(i as f32);
        }

        let a = OwnedMatrix4D::from_vec(a_vec, [2, 2, 2, 2]);
        let b = OwnedMatrix1D::from_vec(vec![1., 2.], [2]);

        let mut ans = OwnedMatrix4D::<f32>::zeros([2, 2, 2, 2]);

        ans.to_view_mut().mul(a.to_view(), b.to_view());

        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    for l in 0..2 {
                        assert_eq!(
                            ans.index_item([i, j, k, l]),
                            a.index_item([i, j, k, l]) * b.index_item([l])
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn mul_4d_2d_dyn() {
        let ones_4d = OwnedMatrixDyn::<f32>::ones([2, 2, 2, 2]);
        let ones_2d = OwnedMatrixDyn::ones([2, 2]);
        let mut ans = OwnedMatrixDyn::zeros([2, 2, 2, 2]);
        ans.to_view_mut().mul(ones_4d.to_view(), ones_2d.to_view());
    }

    #[test]
    fn default_0d_0d() {
        let a = OwnedMatrixDyn::from_vec(vec![10.], &[]);
        let b = OwnedMatrixDyn::from_vec(vec![20.], &[]);
        let mut ans = OwnedMatrixDyn::<f32>::zeros(&[]);
        ans.to_view_mut().mul(a.to_view(), b.to_view());
        assert_eq!(ans.index_item(&[]), 200.);
    }
}

#[cfg(test)]
mod mat_mul {
    use crate::{
        matrix::{IndexItem, OwnedMatrix, ToViewMatrix, ToViewMutMatrix},
        matrix_impl::OwnedMatrix2D,
        operation::zeros::Zeros,
    };

    use super::*;

    #[test]
    fn default() {
        let a = OwnedMatrix2D::from_vec(vec![1., 2., 3., 4., 5., 6.], [2, 3]);
        let b = OwnedMatrix2D::from_vec(
            vec![
                1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12., 13., 14., 15.,
            ],
            [3, 5],
        );
        let mut ans = OwnedMatrix2D::<f32>::zeros([2, 5]);

        ans.to_view_mut().gemm(a.to_view(), b.to_view());
        dbg!(ans.index_item([0, 0]));
        dbg!(ans.index_item([0, 1]));
        dbg!(ans.index_item([1, 0]));
        dbg!(ans.index_item([1, 1]));
        assert_eq!(ans.index_item([0, 0]), 46.);
        assert_eq!(ans.index_item([0, 1]), 52.);
        assert_eq!(ans.index_item([0, 2]), 58.);
        assert_eq!(ans.index_item([0, 3]), 64.);
        assert_eq!(ans.index_item([0, 4]), 70.);
        assert_eq!(ans.index_item([1, 0]), 100.);
        assert_eq!(ans.index_item([1, 1]), 115.);
        assert_eq!(ans.index_item([1, 2]), 130.);
        assert_eq!(ans.index_item([1, 3]), 145.);
        assert_eq!(ans.index_item([1, 4]), 160.);
    }

    #[test]
    fn default_stride_2() {
        let a = OwnedMatrix2D::from_vec(vec![1., 2., 3., 4., 5., 6.], [2, 3]);
        // shape 3 4
        let b = OwnedMatrix2D::from_vec(
            vec![1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12.],
            [3, 4],
        );
        let mut ans = OwnedMatrix2D::<f32>::zeros([2, 4]);

        ans.to_view_mut().gemm(a.to_view(), b.to_view());
        dbg!(ans.index_item([0, 0]));
        dbg!(ans.index_item([0, 1]));
        dbg!(ans.index_item([0, 2]));
        dbg!(ans.index_item([0, 3]));
        dbg!(ans.index_item([1, 0]));
        dbg!(ans.index_item([1, 1]));
        dbg!(ans.index_item([1, 2]));
        dbg!(ans.index_item([1, 3]));

        assert_eq!(ans.index_item([0, 0]), 38.);
        assert_eq!(ans.index_item([0, 1]), 44.);
        assert_eq!(ans.index_item([0, 2]), 50.);
        assert_eq!(ans.index_item([0, 3]), 56.);
        assert_eq!(ans.index_item([1, 0]), 83.);
        assert_eq!(ans.index_item([1, 1]), 98.);
        assert_eq!(ans.index_item([1, 2]), 113.);
        assert_eq!(ans.index_item([1, 3]), 128.);
    }
}
