//! Switch C type definitions

pub type c_schar = i8;
pub type c_uchar = u8;
pub type c_short = i16;
pub type c_ushort = u16;
pub type c_int = i32;
pub type c_uint = u32;
pub type c_float = f32;
pub type c_double = f64;
pub type c_longlong = i64;
pub type c_ulonglong = u64;
pub type intmax_t = i64;
pub type uintmax_t = u64;

pub type size_t = usize;
pub type ptrdiff_t = isize;
pub type intptr_t = isize;
pub type uintptr_t = usize;
pub type ssize_t = isize;

pub type off_t = i64;
pub type c_char = u8;
pub type c_long = i64;
pub type c_ulong = u64;
pub type wchar_t = u32;

pub const INT_MIN: c_int = -2147483648;
pub const INT_MAX: c_int = 2147483647;
