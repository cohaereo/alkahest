use std::{marker::PhantomData, ptr::null_mut};

use umbra3_sys::{
    Umbra_CameraTransform, Umbra_CameraTransform_DepthRange_DEPTHRANGE_ZERO_TO_ONE,
    Umbra_IndexList, Umbra_Matrix4x4, Umbra_MatrixFormat_MF_COLUMN_MAJOR, Umbra_OcclusionBuffer,
    Umbra_OcclusionBuffer_BufferDesc, Umbra_OcclusionBuffer_Format_FORMAT_HISTOGRAM_8BPP,
    Umbra_OcclusionBuffer_Format_FORMAT_NDC_FLOAT,
    Umbra_OcclusionBuffer_VisibilityTestResult_OCCLUDED, Umbra_Query, Umbra_Vector3,
    Umbra_Visibility,
};

use crate::Tome;

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum QueryErrorCode {
    Ok = 0,
    /// Something completely unexpected happened
    GenericError = 1,
    /// Not enough memory was available in the Query context to perform the operation
    OutOfMemory = 2,
    /// An invalid value was passed in
    InvalidArgument = 3,
    /// A tile required to complete the Query was not present in the tome
    SlotdataUnavailable = 4,
    /// A query location was found to be outside of the scene boundaries
    OutsideScene = 5,
    /// No data was given to the Query
    NoTome = 6,
    /// Operation not supported
    UnsupportedOperation = 7,
    /// Path does not exist
    NoPath = 8,
}

pub struct Query<'a> {
    inner: Box<Umbra_Query>,
    _marker: PhantomData<&'a ()>,
}

#[profiling::all_functions]
impl<'a> Query<'a> {
    pub fn new(tome: &'a Tome) -> Self {
        let mut boxed = Box::<umbra3_sys::Umbra_Query>::new_uninit();
        unsafe {
            umbra3_sys::Umbra_Query_Query1(boxed.as_mut_ptr(), tome.0);
        }

        Self {
            inner: unsafe { boxed.assume_init() },
            _marker: PhantomData,
        }
    }

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn query_portal_visibility(
        &mut self,
        flags: u32,
        visibility: &Visibility,
        src: &CameraTransform,
        distance: f32,
        accurate_occlusion_threshold: f32,
        job_index: i32,
        num_jobs: i32,
        grid_width: i32,
    ) -> QueryErrorCode {
        let res = unsafe {
            self.inner.queryPortalVisibility(
                flags,
                &visibility.0,
                &src.0,
                distance,
                accurate_occlusion_threshold,
                null_mut(),
                job_index,
                num_jobs,
                grid_width,
            )
        };

        unsafe { std::mem::transmute(res) }
    }
}

pub struct Visibility(Umbra_Visibility);

impl Visibility {
    pub fn set_output_buffer(&mut self, output_buffer: &mut OcclusionBuffer) {
        unsafe {
            self.0.setOutputBuffer(&mut *output_buffer.0);
        }
    }

    pub fn set_output_clusters(&mut self, output_clusters: &mut IndexList<'_>) {
        unsafe {
            self.0.setOutputClusters(&mut *output_clusters.0);
        }
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Self(unsafe { Umbra_Visibility::new() })
    }
}

pub struct OcclusionBuffer(Box<Umbra_OcclusionBuffer>);

#[profiling::all_functions]
impl OcclusionBuffer {
    pub fn is_aabb_visible(&self, min: [f32; 3], max: [f32; 3]) -> bool {
        unsafe {
            self.0.testAABBVisibility(
                &Umbra_Vector3 { v: min },
                &Umbra_Vector3 { v: max },
                0,
                null_mut(),
            ) != Umbra_OcclusionBuffer_VisibilityTestResult_OCCLUDED
        }
    }

    pub fn width(&self) -> u32 {
        unsafe { self.0.getWidth() as u32 }
    }

    pub fn height(&self) -> u32 {
        unsafe { self.0.getHeight() as u32 }
    }

    pub fn get_desc(&self) -> Umbra_OcclusionBuffer_BufferDesc {
        Umbra_OcclusionBuffer_BufferDesc {
            width: self.width() as i32,
            height: self.height() as i32,
            stride: self.width() as i32,
            format: Umbra_OcclusionBuffer_Format_FORMAT_HISTOGRAM_8BPP,
        }
    }

    pub fn get_buffer(&self, output: &mut [u8], desc: &Umbra_OcclusionBuffer_BufferDesc) {
        #[allow(non_upper_case_globals)]
        match desc.format {
            Umbra_OcclusionBuffer_Format_FORMAT_HISTOGRAM_8BPP => {
                assert!(output.len() == (desc.width * desc.height) as usize);
            }
            Umbra_OcclusionBuffer_Format_FORMAT_NDC_FLOAT => {
                assert!(output.len() == (desc.width * desc.height * 4) as usize);
            }
            u => panic!("Invalid format 0x{u:X} passed"),
        }
        unsafe {
            self.0.getBuffer(output.as_mut_ptr().cast(), desc);
        }
    }
}

impl Default for OcclusionBuffer {
    fn default() -> Self {
        let mut boxed = Box::<umbra3_sys::Umbra_OcclusionBuffer>::new_uninit();
        unsafe {
            umbra3_sys::Umbra_OcclusionBuffer_OcclusionBuffer(boxed.as_mut_ptr());
        }

        Self(unsafe { boxed.assume_init() })
    }
}

pub struct CameraTransform(Umbra_CameraTransform);

impl CameraTransform {
    pub fn new(matrix: [[f32; 4]; 4], position: [f32; 3]) -> Self {
        Self(unsafe {
            Umbra_CameraTransform::new1(
                &Umbra_Matrix4x4 { m: matrix },
                &Umbra_Vector3 { v: position },
                Umbra_CameraTransform_DepthRange_DEPTHRANGE_ZERO_TO_ONE,
                Umbra_MatrixFormat_MF_COLUMN_MAJOR,
            )
        })
    }
}

impl CameraTransform {}

pub struct IndexList<'a>(Box<Umbra_IndexList>, PhantomData<&'a [i32]>);

impl<'a> IndexList<'a> {
    pub fn new(slice: &'a mut [i32]) -> Self {
        let mut boxed = Box::<Umbra_IndexList>::new_uninit();
        unsafe {
            umbra3_sys::Umbra_IndexList_IndexList(
                boxed.as_mut_ptr(),
                slice.as_mut_ptr(),
                slice.len() as i32,
                0,
            );
        }

        Self(unsafe { boxed.assume_init() }, PhantomData)
    }

    pub fn size(&self) -> i32 {
        unsafe { (*self.0).getSize() }
    }
}
