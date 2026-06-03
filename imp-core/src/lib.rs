use std::{ffi::c_void, mem, slice};

#[derive(Clone, Debug)]
pub struct ImpArray<T>
where
    T: Copy,
{
    pub shp: Vec<usize>,
    pub data: Vec<T>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ImpArrayRaw {
    pub len: usize,
    pub dim: usize,
    pub shp_ptr: *mut usize,
    pub data_ptr: *mut c_void,
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
