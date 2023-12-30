pub mod array;

#[repr(C)]
pub struct CShape {
    pub vertices: array::CArray<[f32; 3]>,
    pub indices: array::CArray<u16>,
}

#[no_mangle]
pub extern "C" fn destinyhavok_read_shape_collection(
    data: *mut u8,
    len: usize,
) -> *mut array::CArray<CShape> {
    let data = unsafe { std::slice::from_raw_parts_mut(data, len) };
    let mut cursor = std::io::Cursor::new(data);
    let data = destiny_havok::shape_collection::read_shape_collection(&mut cursor);

    if let Ok(data) = data {
        let data_converted = data
            .into_iter()
            .map(|shape| CShape {
                vertices: array::CArray::new(
                    shape
                        .vertices
                        .into_iter()
                        .map(|v| v.to_array())
                        .collect::<Vec<[f32; 3]>>()
                        .into_boxed_slice(),
                ),
                indices: array::CArray::new(shape.indices.into_boxed_slice()),
            })
            .collect::<Vec<_>>();

        let result = array::CArray::new(data_converted.into_boxed_slice());
        Box::into_raw(Box::new(result))
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn destinyhavok_free_shape_collection(array: *mut array::CArray<CShape>) {
    let _ = unsafe { Box::from_raw(array) };
}
