/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use crate::{AkTransform, AkVector};

impl From<[f32; 3]> for AkVector {
    /// In AkVectors, Y is the up component and Z is the forward component.
    ///
    /// Assumes values in `v` are in XYZ order.
    fn from(v: [f32; 3]) -> Self {
        Self {
            X: v[0],
            Y: v[1],
            Z: v[2],
        }
    }
}

impl From<[f32; 3]> for AkTransform {
    /// Creates an AkTransform at position `p` with default orientation (up pointing up, forward
    /// pointing forward).
    ///
    /// Assumes values in `v` are in XYZ order.
    fn from(p: [f32; 3]) -> Self {
        Self {
            position: AkVector::from(p),
            ..Default::default()
        }
    }
}

impl From<AkVector> for AkTransform {
    /// Creates an AkTransform at position `p` with default orientation (up pointing up, forward
    /// pointing forward).
    fn from(p: AkVector) -> Self {
        Self {
            position: p,
            ..Default::default()
        }
    }
}

impl Default for AkVector {
    /// The nul vector `[0, 0, 0]`.
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl AkVector {
    /// The nul vector `[0, 0, 0]`.
    pub fn new() -> Self {
        Self::default()
    }

    /// The vector `[value, value, value]`.
    pub fn splat<T: Into<f32> + Copy>(value: T) -> Self {
        Self {
            X: value.into(),
            Y: value.into(),
            Z: value.into(),
        }
    }
}

impl Default for AkTransform {
    /// Creates an AkTransform at `[0, 0, 0]` with default orientation (up pointing up, forward
    /// pointing forward).
    ///
    /// *See also*
    /// > - [AkTransform::new]
    fn default() -> Self {
        Self {
            position: AkVector::default(),
            orientationFront: AkVector::from([0., 0., 1.]),
            orientationTop: AkVector::from([0., 1., 0.]),
        }
    }
}

impl AkTransform {
    /// Creates the default AkTransform.
    ///
    /// *See also*
    /// > - [AkTransform::default]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an AkTransform at position `p` with default orientation (up pointing up, forward
    /// pointing forward).
    pub fn from_position<T: Into<AkVector>>(p: T) -> Self {
        AkTransform::from(p.into())
    }
}
