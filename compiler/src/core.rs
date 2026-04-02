use std::ffi::c_void;

pub struct ImpArrayu32 {
    pub shp: Vec<usize>,
    pub data: Vec<u32>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ImpArrayu32Raw {
    pub len: usize,
    pub dim: usize,
    pub shp: *mut usize,
    pub data: *mut u32,
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

impl ImpArrayu32 {
    pub fn as_raw(&mut self) -> ImpArrayu32Raw {
        ImpArrayu32Raw {
            len: self.data.len(),
            dim: self.shp.len(),
            shp: self.shp.as_mut_ptr(),
            data: self.data.as_mut_ptr(),
        }
    }

    /// Copies the C-owned buffers into Rust-owned vectors and frees the C buffers.
    pub unsafe fn from_raw(raw: ImpArrayu32Raw) -> Self {
        let shp = if raw.shp.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(raw.shp, raw.dim).to_vec() }
        };

        let data = if raw.data.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(raw.data, raw.len).to_vec() }
        };

        if !raw.shp.is_null() {
            unsafe { free(raw.shp as *mut c_void) };
        }
        if !raw.data.is_null() {
            unsafe { free(raw.data as *mut c_void) };
        }

        Self { shp, data }
    }
}
