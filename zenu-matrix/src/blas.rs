use crate::num::Num;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlasTrans {
    None,
    Ordinary,
    Conjugate,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlasLayout {
    RowMajor,
    ColMajor,
}

pub trait Blas<T: Num> {
    fn swap(n: usize, x: *mut T, incx: usize, y: *mut T, incy: usize);
    /// x = alpha * x
    fn scal(n: usize, alpha: T, x: *mut T, incx: usize);
    fn axpy(n: usize, alpha: T, x: *const T, incx: usize, y: *mut T, incy: usize);
    fn copy(n: usize, x: *const T, incx: usize, y: *mut T, incy: usize);
    fn dot(n: usize, x: *const T, incx: usize, y: *const T, incy: usize) -> T;
    fn norm2(n: usize, x: *mut T, incx: usize) -> T;
    fn asum(n: usize, x: *const T, incx: usize) -> T;
    fn amax(n: usize, x: *const T, incx: usize) -> usize;
    #[allow(clippy::too_many_arguments)]
    fn gemv(
        layout: BlasLayout,
        trans: BlasTrans,
        m: usize,
        n: usize,
        alpha: T,
        a: *const T,
        lda: usize,
        x: *const T,
        incx: usize,
        beta: T,
        y: *mut T,
        incy: usize,
    );
    #[allow(clippy::too_many_arguments)]
    fn ger(
        layout: BlasLayout,
        m: usize,
        n: usize,
        alpha: T,
        x: *mut T,
        incx: usize,
        y: *mut T,
        incy: usize,
        a: *mut T,
        lda: usize,
    );
    #[allow(clippy::too_many_arguments)]
    fn gemm(
        layout: BlasLayout,
        transa: BlasTrans,
        transb: BlasTrans,
        m: usize,
        n: usize,
        k: usize,
        alpha: T,
        a: *const T,
        lda: usize,
        b: *const T,
        ldb: usize,
        beta: T,
        c: *mut T,
        ldc: usize,
    );
}
