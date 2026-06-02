use std::{ffi::c_void, slice};
use std::mem;

#[derive(Debug)]
pub struct ImpArray<T>
where
    T: Copy,
{
    pub shp: Vec<usize>,
    pub data: Vec<T>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ImpArrayRaw {
    pub len: usize,
    pub dim: usize,
    pub shp: *mut usize,
    pub data: *mut c_void,
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

impl<T> ImpArray<T>
where
    T: Copy,
{
    /// Create a zero-dimensional (scalar) array holding a single value.
    pub fn scalar(value: T) -> Self {
        Self { shp: vec![], data: vec![value] }
    }

    /// Extract the scalar value from a zero-dimensional array.
    ///
    /// Panics if the array has more than zero dimensions.
    pub fn scalar_value(&self) -> T {
        assert!(self.shp.is_empty(), "expected a 0-d (scalar) array");
        self.data[0]
    }

    pub fn into_raw(mut self) -> ImpArrayRaw {
        let len = self.data.len();
        let dim = self.shp.len();
        let shp_ptr = self.shp.as_mut_ptr();
        let data_ptr = self.data.as_mut_ptr() as *mut c_void;
        mem::forget(self.shp);
        mem::forget(self.data);
        ImpArrayRaw { len, dim, shp: shp_ptr, data: data_ptr }
    }

    pub unsafe fn from_raw(raw: ImpArrayRaw) -> Self {
        let shp = if raw.shp.is_null() {
            Vec::new()
        } else {
            let shp = unsafe { slice::from_raw_parts(raw.shp, raw.dim) }.to_vec();
            unsafe { free(raw.shp as *mut c_void) };
            shp
        };

        let data = if raw.data.is_null() {
            Vec::new()
        } else {
            let data = unsafe { slice::from_raw_parts(raw.data as *const T, raw.len) }.to_vec();
            unsafe { free(raw.data) };
            data
        };

        Self { shp, data }
    }
}
