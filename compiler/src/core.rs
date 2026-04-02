use std::{ffi::c_void, ptr};

pub struct ImpArrayu32 {
    pub shp: Vec<usize>,
    pub data: Vec<u32>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ImpArrayu32Raw {
    pub shp: *mut usize,
    pub shp_len: usize,
    pub data: *mut u32,
    pub data_len: usize,
    pub refc: *mut usize,
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

impl ImpArrayu32 {
    pub fn as_raw(&mut self) -> ImpArrayu32Raw {
        ImpArrayu32Raw {
            shp: self.shp.as_mut_ptr(),
            shp_len: self.shp.len(),
            data: self.data.as_mut_ptr(),
            data_len: self.data.len(),
            refc: ptr::null_mut(),
        }
    }

    /// Copies the C-owned buffers into Rust-owned vectors and frees the C buffers.
    pub unsafe fn from_raw(raw: ImpArrayu32Raw) -> Self {
        let shp = if raw.shp.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(raw.shp, raw.shp_len).to_vec() }
        };

        let data = if raw.data.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(raw.data, raw.data_len).to_vec() }
        };

        if !raw.shp.is_null() {
            unsafe { free(raw.shp as *mut c_void) };
        }
        if !raw.data.is_null() {
            unsafe { free(raw.data as *mut c_void) };
        }
        if !raw.refc.is_null() {
            unsafe { free(raw.refc as *mut c_void) };
        }

        Self { shp, data }
    }
}
