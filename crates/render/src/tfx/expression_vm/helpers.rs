use std::ops::{Add, BitAnd, Mul, Shr, Sub};

use glam::{IVec4, UVec4, Vec4, Vec4Swizzles, vec4};

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

fn _trig_helper_vector_pseudo_sin_rotations_clamped(a: Vec4) -> Vec4 {
    a * (a.abs() * -16.0 + 8.0)
}

fn _trig_helper_vector_pseudo_sin_rotations(a: Vec4) -> Vec4 {
    let w = a - a.round(); // wrap to [-0.5, 0.5] range
    _trig_helper_vector_pseudo_sin_rotations_clamped(w)
}

pub fn bytecode_op_triangle(x: Vec4) -> Vec4 {
    let wrapped = x - x.round(); // wrap to [-0.5, 0.5] range
    let abs_wrap = wrapped.abs(); // abs turns into triangle wave between [0, 0.5]

    abs_wrap * 2.0 // scale to [0, 1] range
}

pub fn bytecode_op_jitter(x: Vec4) -> Vec4 {
    let rotations =
        x.xxxx() * Vec4::new(4.67, 2.99, 1.08, 1.35) + Vec4::new(0.52, 0.37, 0.16, 0.79);

    // optimized scaled-sum-of-sines
    let a = rotations - rotations.round(); // wrap to [-0.5, 0.5] range
    let ma = a.abs() * -16.0 + 8.0;
    let sa = a * 0.25;
    let v = sa.dot(ma) + 0.5;

    // hermite smooth interpolation (3*v^2 - 2*v^3)
    let v2 = v * v;
    let jitter_result = (-2.0 * v + 3.0) * v2;

    Vec4::splat(jitter_result)
}

pub fn bytecode_op_wander(x: Vec4) -> Vec4 {
    let rot0 = x.xxxx() * Vec4::new(4.08, 1.02, 3.0 / 5.37, 3.0 / 9.67)
        + Vec4::new(0.92, 0.33, 0.26, 0.54);
    let rot1 = x.xxxx() * Vec4::new(1.83, 3.09, 0.39, 0.87) + Vec4::new(0.12, 0.37, 0.16, 0.79);
    let sines0 = _trig_helper_vector_pseudo_sin_rotations(rot0);
    let sines1 = _trig_helper_vector_pseudo_sin_rotations(rot1) * Vec4::new(0.02, 0.02, 0.28, 0.28);
    let wander_result = 0.5 + sines0.dot(sines1);

    Vec4::splat(wander_result)
}

pub fn bytecode_op_rand(x: Vec4) -> Vec4 {
    // these magic numbers are 1/(prime/1000000)
    let v0 = x.x.floor();
    let mut val0 = Vec4::splat(v0).dot(Vec4::new(
        1.0 / 1.043501,
        1.0 / 0.794471,
        1.0 / 0.113777,
        1.0 / 0.015101,
    ));
    val0 = val0.fract();

    //			val0=	bbs(val0);		// Blum-Blum-Shub randomimzer
    val0 = val0 * val0 * 251.0;
    val0 = val0.fract();

    Vec4::splat(val0)
}

pub fn bytecode_op_rand_smooth(x: Vec4) -> Vec4 {
    let v = x.x;
    let v0 = v.round();
    let v1 = v0 + 1.0;
    let f = v - v0;
    let f2 = f * f;

    // hermite smooth interpolation (3*f^2 - 2*f^3)
    let smooth_f = (-2.0 * f + 3.0) * f2;

    // these magic numbers are 1/(prime/1000000)
    let mut val0 = Vec4::splat(v0).dot(Vec4::new(
        1.0 / 1.043501,
        1.0 / 0.794471,
        1.0 / 0.113777,
        1.0 / 0.015101,
    ));
    let mut val1 = Vec4::splat(v1).dot(Vec4::new(
        1.0 / 1.043501,
        1.0 / 0.794471,
        1.0 / 0.113777,
        1.0 / 0.015101,
    ));

    val0 = val0.fract();
    val1 = val1.fract();

    //			val0=	bbs(val0);		// Blum-Blum-Shub randomimzer
    val0 = val0 * val0 * 251.0;
    val0 = val0.fract();

    //			val10=	bbs(val1);		// Blum-Blum-Shub randomimzer
    val1 = val1 * val1 * 251.0;
    val1 = val1.fract();

    let rand_smooth_result = lerp(val0, val1, smooth_f);

    Vec4::splat(rand_smooth_result)
}

pub fn _trig_helper_vector_sin_rotations_estimate_clamped(a: Vec4) -> Vec4 {
    let y = a * (-16.0 * a.abs() + 8.0);
    y * (0.225 * y.abs() + 0.775)
}

pub fn _trig_helper_vector_sin_rotations_estimate(a: Vec4) -> Vec4 {
    let w = a - a.round(); // wrap to [-0.5, 0.5] range
    _trig_helper_vector_sin_rotations_estimate_clamped(w)
}

pub fn _trig_helper_vector_cos_rotations_estimate(a: Vec4) -> Vec4 {
    _trig_helper_vector_sin_rotations_estimate(a + 0.25)
}

pub fn _trig_helper_vector_sin_cos_rotations_estimate(a: Vec4) -> Vec4 {
    _trig_helper_vector_sin_rotations_estimate(a + Vec4::new(0.0, 0.25, 0.0, 0.25))
}

pub fn bytecode_op_gradient4_const(
    x: Vec4,
    base_color: Vec4,
    cred: Vec4,
    cgreen: Vec4,
    cblue: Vec4,
    calpha: Vec4,
    thresholds: Vec4,
) -> Vec4 {
    // Compute the weighting of each gradient delta based upon the X position of evaluation.
    let c_offsets_from_x = x - thresholds;
    let c_segment_interval = thresholds.yzw().extend(1.0) - thresholds;
    let c_safe_division = if c_offsets_from_x.cmpgt(Vec4::ZERO).all() {
        Vec4::ONE
    } else {
        Vec4::ZERO
    };
    let c_division = if c_offsets_from_x != Vec4::ZERO {
        c_offsets_from_x / c_segment_interval
    } else {
        c_safe_division
    };
    let c_percentages = c_division.clamp(Vec4::ZERO, Vec4::ONE); // Saturate

    // Compute the influence that each of the colors will contribute to the final color.
    let x_influence = cred * c_percentages;
    let y_influence = cgreen * c_percentages;
    let z_influence = cblue * c_percentages;
    let w_influence = calpha * c_percentages;

    // Add the colors into the base color
    base_color
        + Vec4::new(
            Vec4::ONE.dot(x_influence),
            Vec4::ONE.dot(y_influence),
            Vec4::ONE.dot(z_influence),
            Vec4::ONE.dot(w_influence),
        )
}

fn _fake_bitwise_ops_fake_xor(a: Vec4, b: Vec4) -> Vec4 {
    (a + b).rem_euclid(Vec4::splat(2.))
}

#[inline]
fn step(y: Vec4, x: Vec4) -> Vec4 {
    Vec4::select(x.cmpge(y), Vec4::ONE, Vec4::ZERO)
}

pub fn bytecode_op_spline4_const(
    v: Vec4,
    c0: Vec4,
    c1: Vec4,
    c2: Vec4,
    c3: Vec4,
    c4: Vec4,
) -> Vec4 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::*;
        let t0: __m128 = v.into();
        let v264 = _mm_cmple_ps(c4.into(), t0);
        let v265 = _mm_and_ps(
            _mm_add_ps(
                _mm_mul_ps(
                    _mm_add_ps(_mm_mul_ps(t0, c0.into()), c1.into()),
                    _mm_mul_ps(t0, t0),
                ),
                _mm_add_ps(_mm_mul_ps(c2.into(), t0), c3.into()),
            ),
            _mm_xor_ps(
                v264,
                _mm_castsi128_ps(_mm_srli_si128::<4>(_mm_castps_si128(v264))),
            ),
        );
        let v266 = _mm_xor_ps(_mm_shuffle_ps::<78>(v265, v265), v265);
        _mm_xor_ps(_mm_shuffle_ps::<27>(v266, v266), v266).into()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let high = c3 * v + c2;
        let low = c1 * v + c0;

        let v2 = v * v;

        let evaluated_spline = high * v2 + low;

        let threshold_mask = step(c4, v);

        let channel_mask = _fake_bitwise_ops_fake_xor(threshold_mask, threshold_mask.yzww())
            .xyz()
            .extend(threshold_mask.w);

        let spline_result_in_4 = evaluated_spline * channel_mask;

        let spline_result = spline_result_in_4.x
            + spline_result_in_4.y
            + spline_result_in_4.z
            + spline_result_in_4.w;

        Vec4::splat(spline_result)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn bytecode_op_spline8_const(
    x: Vec4,
    c3: Vec4,
    c2: Vec4,
    c1: Vec4,
    c0: Vec4,
    d3: Vec4,
    d2: Vec4,
    d1: Vec4,
    d0: Vec4,
    c_thresholds: Vec4,
    d_thresholds: Vec4,
) -> Vec4 {
    let c_high = c3 * x + c2;
    let c_low = c1 * x + c0;
    let d_high = d3 * x + d2;
    let d_low = d1 * x + d0;

    let x2 = x * x;

    let c_evaluated_spline = c_high * x2 + c_low;
    let d_evaluated_spline = d_high * x2 + d_low;

    let c_threshold_mask = step(c_thresholds, x);
    let d_threshold_mask = step(d_thresholds, x);

    let c_channel_mask = _fake_bitwise_ops_fake_xor(c_threshold_mask, c_threshold_mask.yzww())
        .xyz()
        .extend(c_threshold_mask.w);

    let d_channel_mask = _fake_bitwise_ops_fake_xor(d_threshold_mask, d_threshold_mask.yzww())
        .xyz()
        .extend(d_threshold_mask.w);

    let c_spline_result_in_4 = c_evaluated_spline * c_channel_mask;
    let d_spline_result_in_4 = d_evaluated_spline * d_channel_mask;

    let c_spline_result = c_spline_result_in_4.x
        + c_spline_result_in_4.y
        + c_spline_result_in_4.z
        + c_spline_result_in_4.w;
    let d_spline_result = d_spline_result_in_4.x
        + d_spline_result_in_4.y
        + d_spline_result_in_4.z
        + d_spline_result_in_4.w;

    let spline_result = if d_threshold_mask.x > 0.0 {
        d_spline_result
    } else {
        c_spline_result
    };

    Vec4::splat(spline_result)
}

#[allow(clippy::too_many_arguments)]
pub fn bytecode_op_spline8_chain_const(
    x: Vec4,
    recursion: Vec4,
    c3: Vec4,
    c2: Vec4,
    c1: Vec4,
    c0: Vec4,
    d3: Vec4,
    d2: Vec4,
    d1: Vec4,
    d0: Vec4,
    c_thresholds: Vec4,
    d_thresholds: Vec4,
) -> Vec4 {
    let c_high = c3 * x + c2;
    let c_low = c1 * x + c0;
    let d_high = d3 * x + d2;
    let d_low = d1 * x + d0;

    let x2 = x * x;

    let c_evaluated_spline = c_high * x2 + c_low;
    let d_evaluated_spline = d_high * x2 + d_low;

    let c_threshold_mask = step(c_thresholds, x);
    let d_threshold_mask = step(d_thresholds, x);

    let c_channel_mask = _fake_bitwise_ops_fake_xor(c_threshold_mask, c_threshold_mask.yzww())
        .xyz()
        .extend(c_threshold_mask.w);

    let d_channel_mask = _fake_bitwise_ops_fake_xor(d_threshold_mask, d_threshold_mask.yzww())
        .xyz()
        .extend(d_threshold_mask.w);

    let c_spline_result_in_4 = c_evaluated_spline * c_channel_mask;
    let d_spline_result_in_4 = d_evaluated_spline * d_channel_mask;

    let c_spline_result = c_spline_result_in_4.x
        + c_spline_result_in_4.y
        + c_spline_result_in_4.z
        + c_spline_result_in_4.w;
    let d_spline_result = d_spline_result_in_4.x
        + d_spline_result_in_4.y
        + d_spline_result_in_4.z
        + d_spline_result_in_4.w;

    let spline_result_intermediate = if c_threshold_mask.x > 0.0 {
        c_spline_result
    } else {
        recursion.x
    };

    let spline_result = if d_threshold_mask.x > 0.0 {
        d_spline_result
    } else {
        spline_result_intermediate
    };

    Vec4::splat(spline_result)
}

// TODO(cohae): Fuzztest against original SIMD code
pub fn bytecode_op_24(v15: Vec4) -> Vec4 {
    #[allow(non_snake_case)]
    unsafe {
        use std::arch::x86_64::*;
        let v15 = __m128::from(v15);
        let xmmword_7FF6308FFF50 = __m128::from(Vec4::splat(0.25));
        let xmmword_7FF6309035D0 = _mm_set1_epi32(0x7FFFFFFF);
        let xmmword_7FF630908220 = _mm_set1_epi32(0x4B000000);
        let xmmword_7FF63090E210 = __m128::from(Vec4::splat(0.225));
        let xmmword_7FF63090E260 = __m128::from(Vec4::splat(0.775));
        let xmmword_7FF63090E2D0 = __m128::from(Vec4::splat(8.0));
        let xmmword_7FF63090E350 = __m128::from(Vec4::splat(-16.0));
        let xmmword_7FF631703720 = __m128::from(Vec4::splat(0.0001));

        let v87 = _mm_add_ps(xmmword_7FF6308FFF50, v15);
        let v88 = _mm_cmpgt_epi32(
            xmmword_7FF630908220,
            _mm_and_si128(xmmword_7FF6309035D0, _mm_cvtps_epi32(v15)),
        );
        let v89 = _mm_cmpgt_epi32(
            xmmword_7FF630908220,
            _mm_and_si128(xmmword_7FF6309035D0, _mm_cvtps_epi32(v87)),
        );
        let v90 = _mm_sub_ps(
            v15,
            _mm_or_ps(
                _mm_and_ps(_mm_cvtepi32_ps(_mm_cvtps_epi32(v15)), _mm_cvtepi32_ps(v88)),
                _mm_cvtepi32_ps(_mm_andnot_si128(v88, _mm_cvtps_epi32(v15))),
            ),
        );
        let v91 = _mm_mul_ps(
            _mm_add_ps(
                _mm_mul_ps(
                    _mm_max_ps(_mm_sub_ps(Vec4::ZERO.into(), v90), v90),
                    xmmword_7FF63090E350,
                ),
                xmmword_7FF63090E2D0,
            ),
            v90,
        );
        let v92 = _mm_sub_ps(
            v87,
            _mm_or_ps(
                _mm_and_ps(_mm_cvtepi32_ps(_mm_cvtps_epi32(v87)), _mm_cvtepi32_ps(v89)),
                _mm_cvtepi32_ps(_mm_andnot_si128(v89, _mm_cvtps_epi32(v87))),
            ),
        );
        let v93 = _mm_mul_ps(
            _mm_add_ps(
                _mm_mul_ps(
                    _mm_max_ps(_mm_sub_ps(Vec4::ZERO.into(), v92), v92),
                    xmmword_7FF63090E350,
                ),
                xmmword_7FF63090E2D0,
            ),
            v92,
        );
        let v94 = _mm_mul_ps(
            _mm_add_ps(
                _mm_mul_ps(
                    _mm_max_ps(_mm_sub_ps(Vec4::ZERO.into(), v93), v93),
                    xmmword_7FF63090E210,
                ),
                xmmword_7FF63090E260,
            ),
            v93,
        );
        let v5 = _mm_cmplt_ps(
            xmmword_7FF631703720,
            _mm_max_ps(_mm_sub_ps(Vec4::ZERO.into(), v94), v94),
        );
        let v39 = _mm_div_ps(
            _mm_mul_ps(
                _mm_add_ps(
                    _mm_mul_ps(
                        _mm_max_ps(_mm_sub_ps(Vec4::ZERO.into(), v91), v91),
                        xmmword_7FF63090E210,
                    ),
                    xmmword_7FF63090E260,
                ),
                v91,
            ),
            v94,
        );

        Vec4::from(_mm_and_ps(v39, v5))
    }
}

// TODO(cohae): Fuzztest against original SIMD code
pub fn bytecode_op_25(v: Vec4) -> Vec4 {
    const XMMWORD_7FF73A1FCBD0: UVec4 = UVec4::splat(0x0000007F);
    const XMMWORD_7FF73A1FCBE0: UVec4 = UVec4::splat(0x007FFFFF);
    const XMMWORD_7FF73A1FCBF0: Vec4 = Vec4::splat(f32::from_bits(0x34000000));
    const XMMWORD_7FF73A1FCBA0: Vec4 = Vec4::new(1.4232545, -0.585_421_1, 0.16216666, 0.0);
    const XMMWORD_7FF73A1F8070: UVec4 = UVec4::splat(0x7F800000);

    // let   v1 = Vec4::mul(
    //     _mm_cvtepi32_ps(_mm_and_si128(_mm_load_si128((const __m128i *)&XMMWORD_7FF73A1FCBE0), *a1)),
    //     XMMWORD_7FF73A1FCBF0);

    let v1 = Vec4::mul(
        Vec4::from_bits_uvec4(XMMWORD_7FF73A1FCBE0 & v.as_bits_uvec4()),
        XMMWORD_7FF73A1FCBF0,
    );

    let rhs = (IVec4::sub(
        UVec4::shr(
            UVec4::bitand(XMMWORD_7FF73A1F8070, v.as_bits_uvec4()),
            0x17i32,
        )
        .as_ivec4(),
        XMMWORD_7FF73A1FCBD0.as_ivec4(),
    ))
    .as_vec4();

    Vec4::add(
        Vec4::add(
            Vec4::mul(
                Vec4::add(
                    XMMWORD_7FF73A1FCBA0.yyyy(),
                    Vec4::mul(XMMWORD_7FF73A1FCBA0.zzzz(), v1),
                ),
                Vec4::mul(v1, v1),
            ),
            Vec4::mul(XMMWORD_7FF73A1FCBA0.xxxx(), v1),
        ),
        rhs,
    )
}

pub fn bytecode_op_unk3b_const(input: Vec4, constants: &[Vec4]) -> Vec4 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::{
            __m128, _mm_add_ps, _mm_and_ps, _mm_andnot_ps, _mm_cmple_ps, _mm_cmplt_ps, _mm_div_ps,
            _mm_max_ps, _mm_min_ps, _mm_mul_ps, _mm_or_ps, _mm_set1_ps, _mm_shuffle_ps, _mm_sub_ps,
        };

        let f32_mask = f32::from_bits(u32::MAX);
        let mask_all = _mm_set1_ps(f32_mask);
        let mask_xyz = __m128::from(vec4(f32_mask, f32_mask, f32_mask, 0.0));
        let mask_x = __m128::from(vec4(f32_mask, 0.0, 0.0, 0.0));
        let mask_y = __m128::from(vec4(0.0, f32_mask, 0.0, 0.0));
        let mask_z = __m128::from(vec4(0.0, 0.0, f32_mask, 0.0));
        let mask_w = __m128::from(vec4(0.0, 0.0, 0.0, f32_mask));
        let zero = _mm_set1_ps(0.0);
        let one = _mm_set1_ps(1.0);
        let epsilon = _mm_set1_ps(0.0001);

        let v4 = __m128::from(constants[10]);
        let v5 = __m128::from(constants[9]);
        let v6 = __m128::from(constants[0]);
        let v7 = _mm_sub_ps(input.into(), v5);
        let v8 = _mm_sub_ps(input.into(), v4);
        let v9 = _mm_sub_ps(
            _mm_or_ps(
                _mm_and_ps(_mm_and_ps(_mm_shuffle_ps(v4, v4, 57), mask_all), mask_xyz),
                _mm_andnot_ps(mask_xyz, one),
            ),
            v4,
        );
        let v10 = _mm_sub_ps(
            _mm_add_ps(
                _mm_or_ps(
                    _mm_and_ps(_mm_and_ps(_mm_shuffle_ps(v4, v4, 0), mask_w), mask_all),
                    _mm_andnot_ps(mask_all, one),
                ),
                _mm_and_ps(_mm_shuffle_ps(v5, v5, 57), mask_xyz),
            ),
            v5,
        );
        let v11 = _mm_cmplt_ps(epsilon, _mm_max_ps(_mm_sub_ps(zero, v10), v10));
        let v12 = _mm_cmplt_ps(epsilon, _mm_max_ps(_mm_sub_ps(zero, v9), v9));
        let v13 = _mm_min_ps(
            _mm_max_ps(
                _mm_or_ps(
                    _mm_andnot_ps(v11, _mm_and_ps(_mm_cmple_ps(zero, v7), one)),
                    _mm_and_ps(_mm_div_ps(v7, v10), v11),
                ),
                zero,
            ),
            one,
        );
        let v14 = _mm_min_ps(
            _mm_max_ps(
                _mm_or_ps(
                    _mm_andnot_ps(v12, _mm_and_ps(_mm_cmple_ps(zero, v8), one)),
                    _mm_and_ps(_mm_div_ps(v8, v9), v12),
                ),
                zero,
            ),
            one,
        );
        let v15 = _mm_mul_ps(
            _mm_add_ps(
                _mm_mul_ps(__m128::from(constants[1]), v13),
                _mm_mul_ps(__m128::from(constants[5]), v14),
            ),
            one,
        );
        let v16 = _mm_add_ps(_mm_shuffle_ps(v15, v15, 78), v15);
        let v17 = _mm_add_ps(
            _mm_or_ps(
                _mm_and_ps(
                    _mm_and_ps(_mm_add_ps(_mm_shuffle_ps(v16, v16, 147), v16), mask_x),
                    mask_all,
                ),
                _mm_andnot_ps(mask_all, one),
            ),
            v6,
        );
        let v18 = _mm_add_ps(
            _mm_mul_ps(__m128::from(constants[2]), v13),
            _mm_mul_ps(__m128::from(constants[6]), v14),
        );
        let v19 = _mm_mul_ps(v18, one);
        let v20 = _mm_add_ps(_mm_shuffle_ps(v19, v19, 78), v19);
        let v21 = _mm_add_ps(
            _mm_or_ps(
                _mm_and_ps(
                    _mm_and_ps(_mm_add_ps(_mm_shuffle_ps(v20, v20, 147), v20), mask_y),
                    mask_all,
                ),
                _mm_andnot_ps(mask_all, one),
            ),
            v17,
        );
        let v22 = _mm_mul_ps(__m128::from(constants[3]), v13);
        let v23 = _mm_mul_ps(__m128::from(constants[7]), v14);
        let v24 = _mm_mul_ps(_mm_add_ps(v22, v23), one);
        let v25 = _mm_add_ps(_mm_shuffle_ps(v24, v24, 78), v24);
        let v27 = _mm_mul_ps(__m128::from(constants[4]), v13);
        let v28 = _mm_add_ps(
            _mm_or_ps(
                _mm_and_ps(
                    _mm_and_ps(_mm_add_ps(_mm_shuffle_ps(v25, v25, 147), v25), mask_z),
                    mask_all,
                ),
                _mm_andnot_ps(mask_all, one),
            ),
            v21,
        );
        let v29 = _mm_mul_ps(__m128::from(constants[8]), v14);
        let v30 = _mm_mul_ps(_mm_add_ps(v27, v29), one);
        let v31 = _mm_add_ps(_mm_shuffle_ps(v30, v30, 78), v30);
        let result = _mm_add_ps(
            _mm_or_ps(
                _mm_and_ps(
                    _mm_and_ps(_mm_add_ps(_mm_shuffle_ps(v31, v31, 147), v31), mask_w),
                    mask_all,
                ),
                _mm_andnot_ps(mask_all, one),
            ),
            v28,
        );

        Vec4::from(result)
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        // warn!("bytecode_op_unk3b_const is not implemented for this architecture");
        Vec4::ZERO
    }
}

trait Vec4SimdExt {
    fn as_bits_uvec4(&self) -> UVec4;
    fn from_bits_uvec4(bits: UVec4) -> Self;
}

impl Vec4SimdExt for Vec4 {
    fn as_bits_uvec4(&self) -> UVec4 {
        UVec4::from_slice(bytemuck::cast_slice(&self.to_array()))
    }

    fn from_bits_uvec4(bits: UVec4) -> Self {
        Vec4::from_slice(bytemuck::cast_slice(&bits.to_array()))
    }
}
