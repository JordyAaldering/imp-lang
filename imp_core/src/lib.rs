use std::{ffi::c_void, mem, slice};

#[derive(Clone, Debug)]
pub struct ImpArray<T>
where
    T: Copy,
{
    shp: Vec<usize>,
    data: Vec<T>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ImpArrayRaw {
    len: usize,
    dim: usize,
    shp_ptr: *mut usize,
    data_ptr: *mut c_void,
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

impl<T> ImpArray<T>
where
    T: Copy,
{
    /// Create a new array with the given shape and data.
    ///
    /// The length of the data must match the product of the shape dimensions.
    pub fn new(shp: Vec<usize>, data: Vec<T>) -> Self {
        debug_assert_eq!(shp.iter().product::<usize>(), data.len(), "Shape and data length mismatch");
        Self { shp, data }
    }

    /// Create a new vector (1-dimensional array) with the given length and data.
    ///
    /// The length of the data must match the specified length.
    pub fn vector(len: usize, data: Vec<T>) -> Self {
        debug_assert_eq!(len, data.len(), "Length and data length mismatch");
        Self { shp: vec![len], data }
    }

    /// Create a scalar (0-dimensional array) from the given value.
    pub fn scalar(value: T) -> Self {
        Self { shp: vec![], data: vec![value] }
    }

    /// Extract the scalar value from a zero-dimensional array.
    ///
    /// Panics if the array has more than zero dimensions.
    pub fn unwrap_scalar(&self) -> T {
        debug_assert!(self.is_scalar(), "Expected a 0-dimensional (scalar) array");
        self.data[0]
    }

    pub fn is_scalar(&self) -> bool {
        self.shp.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn dim(&self) -> usize {
        self.shp.len()
    }

    pub fn shape(&self) -> &[usize] {
        &self.shp
    }

    pub fn extent(&self, axis: usize) -> usize {
        debug_assert!(axis < self.shp.len(), "Axis out of bounds");
        self.shp[axis]
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }

    pub fn into_raw(mut self) -> ImpArrayRaw {
        let len = self.data.len();
        let dim = self.shp.len();
        let shp_ptr = self.shp.as_mut_ptr();
        let data_ptr = self.data.as_mut_ptr() as *mut c_void;
        mem::forget(self.shp);
        mem::forget(self.data);
        ImpArrayRaw { len, dim, shp_ptr, data_ptr }
    }

    pub unsafe fn from_raw(raw: ImpArrayRaw) -> Self {
        let shp = if raw.shp_ptr.is_null() {
            Vec::new()
        } else {
            let shp = unsafe { slice::from_raw_parts(raw.shp_ptr, raw.dim) }.to_vec();
            unsafe { free(raw.shp_ptr as *mut c_void) };
            shp
        };

        let data = if raw.data_ptr.is_null() {
            Vec::new()
        } else {
            let data = unsafe { slice::from_raw_parts(raw.data_ptr as *const T, raw.len) }.to_vec();
            unsafe { free(raw.data_ptr) };
            data
        };

        Self { shp, data }
    }
}
