use std::ffi::c_void;

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
    pub fn as_raw(&mut self) -> ImpArrayRaw {
        ImpArrayRaw {
            len: self.data.len(),
            dim: self.shp.len(),
            shp: self.shp.as_mut_ptr(),
            data: self.data.as_mut_ptr() as *mut c_void,
        }
    }

    pub unsafe fn from_raw(raw: ImpArrayRaw) -> Self {
        let shp = if raw.shp.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(raw.shp, raw.dim).to_vec() }
        };

        let data = if raw.data.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(raw.data as *const T, raw.len).to_vec() }
        };

        if !raw.shp.is_null() {
            unsafe { free(raw.shp as *mut c_void) };
        }

        if !raw.data.is_null() {
            unsafe { free(raw.data) };
        }

        Self { shp, data }
    }
}
