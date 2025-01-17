//! # Clip and ClipAssign Traits
//!
//! This module provides the `Clip` and `ClipAssign` traits for clipping matrix values within a specified range.
//!
//! ## ClipAssign Trait
//!
//! The `ClipAssign` trait defines a method for clipping the values of a mutable matrix in-place.
//!
//! - `clip_assign(self, min: T, max: T)`: Clips the values of the matrix to be within the range `[min, max]`.
//!
//! ## Clip Trait
//!
//! The `Clip` trait defines a method for creating a new matrix with the values clipped within a specified range.
//!
//! - `clip(&self, min: T, max: T) -> Matrix<OwnedMem<T>, Self::Dim>`: Creates a new matrix with the values clipped to be within the range `[min, max]`.
//!
//! ## Implementations
//!
//! The `ClipAssign` trait is implemented for `Matrix<SM, D>` where:
//! - `T` is the element type and must implement the `Num` trait.
//! - `SM` is the storage type and must implement the `ToViewMutMemory` trait with `Item = T`.
//! - `D` is the dimension type and must implement the `DimTrait`.
//!
//! The implementation of `clip_assign` handles different cases based on the number of dimensions of the matrix:
//! - For 1-dimensional matrices, it calls the `clip_assign_kernel_cpu` function to perform the clipping using CPU.
//! - For 0-dimensional matrices, it is currently unimplemented.
//! - For multi-dimensional matrices, it iterates over the first dimension and recursively calls `clip_assign` on the sub-matrices.
//!
//! The `Clip` trait is implemented for `Matrix<SM, D>` where:
//! - `T` is the element type and must implement the `Num` trait.
//! - `SM` is the storage type and must implement the `ToViewMemory` trait with `Item = T`.
//! - `D` is the dimension type and must implement the `DimTrait`.
//!
//! The implementation of `clip` creates a new matrix with the same shape as the input matrix, copies the values from the input matrix, and then calls `clip_assign` on the new matrix to perform the clipping.
//!
//! ## Functions
//!
//! - `clip_assign_kernel_cpu<T: Num, M: ViewMut<Item = T>, D: DimTrait>(result: &mut Matrix<M, D>, min: T, max: T)`: Performs the clipping operation on a 1-dimensional matrix using CPU.
//!
//! ## Testing
//!
//! The module includes unit tests in the `clip` submodule to verify the correctness of the `Clip` trait implementation. The tests cover different scenarios, such as 1-dimensional, 2-dimensional, and 3-dimensional matrices, and compare the results against expected values.
//!
//! ## Example
//!
//! ```rust
//! use zenu_matrix::{matrix::OwnedMatrix, matrix_impl::OwnedMatrixDyn, operation::clip::Clip};
//!
//! let a = OwnedMatrixDyn::from_vec(vec![1.0, 2.0, 3.0, 4.0], [2, 2]);
//! let b = a.clip(2.0, 3.0);
//! ```

use super::copy_from::CopyFrom;
use crate::{
    constructor::zeros::Zeros,
    dim::{DimDyn, DimTrait},
    index::Index0D,
    matrix::{
        IndexAxisDyn, IndexAxisMutDyn, MatrixBase, OwnedMatrix, ToViewMatrix, ToViewMutMatrix,
    },
    matrix_impl::{Matrix, OwnedMatrixDyn},
    memory::{ToViewMemory, ToViewMutMemory, ViewMut},
    memory_impl::{OwnedMem, ViewMem},
    num::Num,
};

/// Trait for clipping the values of a mutable matrix in-place.
pub trait ClipAssign<T: Num> {
    /// Clips the values of the matrix to be within the range `[min, max]`.
    fn clip_assign(self, min: T, max: T);
}

/// Trait for creating a new matrix with the values clipped within a specified range.
pub trait Clip<T: Num>: MatrixBase {
    /// Creates a new matrix with the values clipped to be within the range `[min, max]`.
    fn clip(&self, min: T, max: T) -> Matrix<OwnedMem<T>, Self::Dim>;
}

impl<T: Num, SM: ToViewMemory + ToViewMutMemory<Item = T>, D: DimTrait> ClipAssign<T>
    for Matrix<SM, D>
{
    fn clip_assign(mut self, min: T, max: T) {
        if self.shape().len() == 1 {
            clip_assign_kernel_cpu(&mut self.to_view_mut(), min, max);
        } else if self.shape().len() == 0 {
            unimplemented!();
        } else {
            let mut s = self.into_dyn_dim();
            for i in 0..s.shape()[0] {
                s.index_axis_mut_dyn(Index0D::new(i)).clip_assign(min, max);
            }
        }
    }
}

impl<T: Num, SM: ToViewMemory<Item = T>, D: DimTrait> Clip<T> for Matrix<SM, D> {
    fn clip(&self, min: T, max: T) -> Matrix<OwnedMem<T>, Self::Dim> {
        let mut result = Matrix::<OwnedMem<T>, Self::Dim>::zeros(self.shape());
        let v_m = result.to_view_mut();
        let mut v_m = v_m.into_dyn_dim();
        v_m.copy_from(&self.to_view().into_dyn_dim());
        v_m.clip_assign(min, max);
        result
    }
}

fn clip_assign_kernel_cpu<T: Num, M: ViewMut<Item = T>, D: DimTrait>(
    result: &mut Matrix<M, D>,
    min: T,
    max: T,
) {
    let stride = result.stride()[0];
    let len = result.shape()[0];
    let slice = result.as_mut_slice();
    for idx in 0..len {
        let mut x = slice[idx * stride];
        if x < min {
            x = min;
        } else if x > max {
            x = max;
        }
        slice[idx * stride] = x;
    }
}

pub fn clip_filter<T: Num, M: ToViewMemory<Item = T>>(
    input: Matrix<M, DimDyn>,
    max: T,
    min: T,
) -> Matrix<OwnedMem<T>, DimDyn> {
    let mut output = OwnedMatrixDyn::zeros(input.shape());
    if input.shape().len() == 1 {
        return inner(input.to_view(), max, min);
    } else if input.shape().is_empty() {
        unimplemented!();
    } else {
        let s = input.to_view().into_dyn_dim();
        let mut output = output.to_view_mut().into_dyn_dim();
        for i in 0..s.shape()[0] {
            let tmp = s.index_axis_dyn(Index0D::new(i));
            let mut slice = output.index_axis_mut_dyn(Index0D::new(i));
            slice.copy_from(&clip_filter(tmp, max, min).to_view());
        }
    }
    output
}

fn inner<T: Num>(input: Matrix<ViewMem<T>, DimDyn>, max: T, min: T) -> OwnedMatrixDyn<T> {
    let len = input.shape()[0];
    let input_stride = input.stride()[0];
    let input_slice = input.as_slice();
    let mut output_vec = Vec::with_capacity(len);
    for i in 0..len {
        let tmp = input_slice[i * input_stride];
        if min < tmp || tmp < max {
            output_vec.push(T::zero());
        } else {
            output_vec.push(T::one());
        }
    }
    OwnedMatrixDyn::from_vec(output_vec, input.shape())
}

#[cfg(test)]
mod clip {
    use crate::{
        matrix::{OwnedMatrix, ToViewMutMatrix},
        matrix_impl::OwnedMatrixDyn,
        operation::{
            asum::Asum,
            clip::{Clip, ClipAssign},
        },
    };

    #[test]
    fn clip_1d() {
        let a = OwnedMatrixDyn::from_vec(vec![1.0, 2.0, 3.0, 4.0], [4]);
        let b = a.clip(2.0, 3.0);
        assert_eq!(b.as_slice(), &[2.0, 2.0, 3.0, 3.0]);
    }

    #[test]
    fn clip_2d() {
        let a = OwnedMatrixDyn::from_vec(vec![1.0, 2.0, 3.0, 4.0], [2, 2]);
        let b = a.clip(2.0, 3.0);
        let ans = OwnedMatrixDyn::from_vec(vec![2.0, 2.0, 3.0, 3.0], [2, 2]);
        let diff = b - ans;
        let diff_asum = diff.asum();
        assert_eq!(diff_asum, 0.0);
    }

    #[test]
    fn clip_3d() {
        // shape 3 x 3 x 3
        let a = OwnedMatrixDyn::from_vec(
            vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0,
            ],
            [3, 3, 3],
        );
        let b = a.clip(2.0, 3.0);
        let ans = OwnedMatrixDyn::from_vec(
            vec![
                2.0, 2.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0,
                3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0,
            ],
            [3, 3, 3],
        );
        let diff = b - ans;
        let diff_asum = diff.asum();
        assert_eq!(diff_asum, 0.0);
    }

    #[test]
    fn clip_assign_2d_2() {
        let mut a = OwnedMatrixDyn::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], [2, 3]);
        a.to_view_mut().clip_assign(2.0, 4.0);
        let ans = OwnedMatrixDyn::from_vec(vec![2.0, 2.0, 3.0, 4.0, 4.0, 4.0], [2, 3]);
        let diff = a - ans;
        let diff_asum = diff.asum();
        assert_eq!(diff_asum, 0.0);
    }
}
