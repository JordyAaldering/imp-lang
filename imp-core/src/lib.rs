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

pub type ImpDynBool = ImpDyn<bool>;
pub type ImpDynI32 = ImpDyn<i32>;
pub type ImpDynI64 = ImpDyn<i64>;
pub type ImpDynU32 = ImpDyn<u32>;
pub type ImpDynU64 = ImpDyn<u64>;
pub type ImpDynUsize = ImpDyn<usize>;
pub type ImpDynF32 = ImpDyn<f32>;
pub type ImpDynF64 = ImpDyn<f64>;

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

    pub unsafe fn into_array_or_scalar(self) -> ImpArrayOrScalar<T> {
        if self.is_array {
            let raw = unsafe { self.data.array };
            ImpArrayOrScalar::Array(unsafe { ImpArray::<T>::from_raw(raw) })
        } else {
            ImpArrayOrScalar::Scalar(unsafe { self.data.scalar })
        }
    }
}

pub fn expect_scalar<T: Copy>(value: ImpArrayOrScalar<T>) -> T {
    match value {
        ImpArrayOrScalar::Scalar(v) => v,
        ImpArrayOrScalar::Array(_) => panic!("expected a scalar"),
    }
}

pub fn expect_array<T: Copy>(value: ImpArrayOrScalar<T>) -> ImpArray<T> {
    match value {
        ImpArrayOrScalar::Array(v) => v,
        ImpArrayOrScalar::Scalar(_) => panic!("expected an array"),
    }
}
