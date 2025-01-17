use zenu_matrix::{
    constructor::zeros::Zeros,
    dim::{DimDyn, DimTrait},
    matrix::{MatrixBase, ToViewMatrix, ToViewMutMatrix},
    matrix_impl::{Matrix, OwnedMatrixDyn},
    memory_impl::{OwnedMem, ViewMem},
    num::Num,
    operation::{
        basic_operations::MatrixAddAssign,
        mul::Gemm,
        reshape::{Reshape, ReshapeNoAlloc},
        transpose::TransposeInplace,
    },
};

use super::im2col::{im2col, Im2ColRes};

pub(super) fn conv2d_out_size(
    img_shape: &[usize],
    kernel_shape: &[usize],
    padding: (usize, usize),
    stride: (usize, usize),
) -> [usize; 4] {
    let (b, h, w) = (img_shape[0], img_shape[2], img_shape[3]);
    let (oc, kh, kw) = (kernel_shape[0], kernel_shape[2], kernel_shape[3]);
    let (ph, pw) = padding;
    let (sh, sw) = stride;
    let (h, w) = ((h + 2 * ph - kh) / sh + 1, (w + 2 * pw - kw) / sw + 1);
    [b, oc, h, w]
}

pub(super) fn conv2d_inner<T: Num>(
    img: Matrix<ViewMem<T>, DimDyn>,
    kernel: Matrix<ViewMem<T>, DimDyn>,
    bias: Option<Matrix<OwnedMem<T>, DimDyn>>,
    padding: (usize, usize),
    stride: (usize, usize),
) -> Matrix<OwnedMem<T>, DimDyn> {
    let batch_size = img.shape()[0];
    let kernel_shape = kernel.shape();
    let kernel_h_w = (kernel_shape[2], kernel_shape[3]);
    let Im2ColRes { col, out_size } = im2col(img, kernel_h_w, stride, padding);
    // kenrnel shape is [out_channel, in_channel, kernel_h, kernel_w]
    let kernel = kernel.reshape([
        kernel.shape()[0],
        kernel.shape().num_elm() / kernel.shape()[0],
    ]);
    let mut result = OwnedMatrixDyn::zeros([kernel_shape[0], col.shape()[1]]);
    result.to_view_mut().gemm(kernel, col.to_view());
    let mut result = result
        .reshape([kernel_shape[0], batch_size, out_size.0 * out_size.1])
        .transpose_swap_index_inplace(0, 1)
        .reshape_no_alloc_owned([batch_size, kernel_shape[0], out_size.0, out_size.1]);
    if let Some(bias) = bias {
        result.add_assign(bias.to_view());
    }
    result
}

#[cfg(test)]
mod conv2d {
    use zenu_matrix::{
        matrix::{OwnedMatrix, ToViewMatrix},
        matrix_impl::OwnedMatrixDyn,
        operation::asum::Asum,
    };

    use super::conv2d_inner;

    #[test]
    fn conv2d_5x5im_3x3_kernel_0x0_pad_1x1_stride() {
        let kernel =
            OwnedMatrixDyn::from_vec(vec![1., 2., 3., 4., 5., 6., 7., 8., 9.], [1, 1, 3, 3]);
        let img = (1..26).map(|x| x as f32).collect::<Vec<f32>>();
        let img = OwnedMatrixDyn::from_vec(img, [1, 1, 5, 5]);
        let out = conv2d_inner(img.to_view(), kernel.to_view(), None, (0, 0), (1, 1));
        let ans = OwnedMatrixDyn::from_vec(
            vec![411., 456., 501., 636., 681., 726., 861., 906., 951.],
            [1, 1, 3, 3],
        );
        assert!((out - ans).asum() < 1e-6);
    }

    #[test]
    fn conv2d_5x5xim_3x3_kernel_1x1_pad_1x1_stride() {
        let kernel =
            OwnedMatrixDyn::from_vec(vec![1., 2., 3., 4., 5., 6., 7., 8., 9.], [1, 1, 3, 3]);
        let img = (1..26).map(|x| x as f32).collect::<Vec<f32>>();
        let img = OwnedMatrixDyn::from_vec(img, [1, 1, 5, 5]);
        let out = conv2d_inner(img.to_view(), kernel.to_view(), None, (1, 1), (1, 1));
        let ans = OwnedMatrixDyn::from_vec(
            vec![
                128., 202., 241., 280., 184., 276., 411., 456., 501., 318., 441., 636., 681., 726.,
                453., 606., 861., 906., 951., 588., 320., 436., 457., 478., 280.,
            ],
            [1, 1, 5, 5],
        );
        assert!((out - ans).asum() < 1e-6);
    }

    #[test]
    fn conv2d_5x5im_3x3_kernel_1x1_pad_2x2_stride() {
        let kernel =
            OwnedMatrixDyn::from_vec(vec![1., 2., 3., 4., 5., 6., 7., 8., 9.], [1, 1, 3, 3]);
        let img = (1..26).map(|x| x as f32).collect::<Vec<f32>>();
        let img = OwnedMatrixDyn::from_vec(img, [1, 1, 5, 5]);
        let out = conv2d_inner(img.to_view(), kernel.to_view(), None, (1, 1), (2, 2));
        let ans = OwnedMatrixDyn::from_vec(
            vec![128., 241., 184., 441., 681., 453., 320., 457., 280.],
            [1, 1, 3, 3],
        );
        assert!((out - ans).asum() < 1e-6);
    }

    #[test]
    fn conv2d_5x5_im_3ch_2_batch_1x1_kernel_1x1_pad_1x1_stride() {
        let input = (1..=(5 * 5 * 3 * 2))
            .map(|x| x as f32)
            .collect::<Vec<f32>>();
        let input = OwnedMatrixDyn::from_vec(input, [2, 3, 5, 5]);
        let kernel = (1..=(3 * 3 * 3 * 4))
            .map(|x| x as f32)
            .collect::<Vec<f32>>();
        let kernel = OwnedMatrixDyn::from_vec(kernel, [4, 3, 3, 3]);
        let out = conv2d_inner(input.to_view(), kernel.to_view(), None, (1, 1), (1, 1));
        let ans = vec![
            7416., 11010., 11289., 11568., 7608., 11106., 16434., 16812., 17190., 11268., 12411.,
            18324., 18702., 19080., 12483., 13716., 20214., 20592., 20970., 13698., 8712., 12792.,
            13017., 13242., 8616., 16812., 25347., 26112., 26877., 17976., 26415., 39762., 40869.,
            41976., 28035., 30150., 45297., 46404., 47511., 31680., 33885., 50832., 51939., 53046.,
            35325., 22968., 34419., 35130., 35841., 23844., 26208., 39684., 40935., 42186., 28344.,
            41724., 63090., 64926., 66762., 44802., 47889., 72270., 74106., 75942., 50877., 54054.,
            81450., 83286., 85122., 56952., 37224., 56046., 57243., 58440., 39072., 35604., 54021.,
            55758., 57495., 38712., 57033., 86418., 88983., 91548., 61569., 65628., 99243.,
            101808., 104373., 70074., 74223., 112068., 114633., 117198., 78579., 51480., 77673.,
            79356., 81039., 54300., 21816., 31935., 32214., 32493., 21108., 30681., 44784., 45162.,
            45540., 29493., 31986., 46674., 47052., 47430., 30708., 33291., 48564., 48942., 49320.,
            31923., 20412., 29667., 29892., 30117., 19416., 55512., 82722., 83487., 84252., 55776.,
            82440., 122787., 123894., 125001., 82710., 86175., 128322., 129429., 130536., 86355.,
            89910., 133857., 134964., 136071., 90000., 58968., 87744., 88455., 89166., 58944.,
            89208., 133509., 134760., 136011., 90444., 134199., 200790., 202626., 204462., 135927.,
            140364., 209970., 211806., 213642., 142002., 146529., 219150., 220986., 222822.,
            148077., 97524., 145821., 147018., 148215., 98472., 122904., 184296., 186033., 187770.,
            125112., 185958., 278793., 281358., 283923., 189144., 194553., 291618., 294183.,
            296748., 197649., 203148., 304443., 307008., 309573., 206154., 136080., 203898.,
            205581., 207264., 138000.,
        ];
        let ans = OwnedMatrixDyn::from_vec(ans, [2, 4, 5, 5]);
        assert!((out - ans).asum() < 1e-6);
    }
}
