use std::{ffi::c_void, slice};

#[derive(Debug)]
pub enum ImpArrayOrScalar<T>
where
    T: Copy,
{
    Array(ImpArray<T>),
    Scalar(T),
}

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

#[repr(C)]
#[derive(Clone, Copy)]
pub union ImpDynData<T>
where
    T: Copy,
{
    pub scalar: T,
    pub array: ImpArrayRaw,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ImpDyn<T>
where
    T: Copy,
{
    pub is_array: bool,
    pub data: ImpDynData<T>,
}

pub type ImpDynU32 = ImpDyn<u32>;
pub type ImpDynUsize = ImpDyn<usize>;
pub type ImpDynBool = ImpDyn<bool>;

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

    /// # Safety
    ///
    /// `raw` must originate from this runtime's allocation conventions:
    /// - `raw.shp` points to a heap allocation with exactly `raw.dim` `usize` elements
    /// - `raw.data` points to a heap allocation with exactly `raw.len` `T` elements
    /// - both pointers are either null or valid for reads of those lengths
    /// - both pointers were allocated with a compatible allocator for `free`
    ///
    /// This function takes ownership of both buffers and frees them exactly once.
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

impl<T> ImpDyn<T>
where
    T: Copy,
{
    pub fn from_scalar(value: T) -> Self {
        Self {
            is_array: false,
            data: ImpDynData { scalar: value },
        }
    }

    pub fn from_array_raw(raw: ImpArrayRaw) -> Self {
        Self {
            is_array: true,
            data: ImpDynData { array: raw },
        }
    }

    /// # Safety
    ///
    /// If `is_array` is true, `data.array` must satisfy `ImpArray::<T>::from_raw` safety.
    pub unsafe fn into_array_or_scalar(self) -> ImpArrayOrScalar<T> {
        if self.is_array {
            let raw = unsafe { self.data.array };
            ImpArrayOrScalar::Array(unsafe { ImpArray::<T>::from_raw(raw) })
        } else {
            ImpArrayOrScalar::Scalar(unsafe { self.data.scalar })
        }
    }
}
