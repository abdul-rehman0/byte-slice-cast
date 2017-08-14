// Copyright (C) 2017 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

//! Safely cast bytes slices from/to slices of built-in fundamental numeric types.
//!
//! The provided traits here allow safe casting between byte slices and slices of fundamental
//! numeric types, like integers and floating point numbers. During casting, checks are performed
//! to ensure that the output slice is safe to use: the input slice must be properly aligned for
//! the output type and contain an integer number of values.
//!
//! The content of the output slice will be bitwise equivalent to the input slice, as such extra
//! care has to be taken with regard to endianness.
//!
//! # Example
//! ```
//! # extern crate byte_slice_cast;
//! use byte_slice_cast::*;
//!
//! # fn main() {
//! let slice = [1u8, 2u8, 3u8, 4u8, 5u8, 6u8];
//! let converted_slice = slice.as_slice_of::<u16>().unwrap();
//!
//! if cfg!(target_endian = "big") {
//!     assert_eq!(converted_slice, &[0x0102, 0x0304, 0x0506]);
//! } else {
//!     assert_eq!(converted_slice, &[0x0201, 0x0403, 0x0605]);
//! }
//!
//! let converted_back_slice = converted_slice.as_byte_slice().unwrap();
//! assert_eq!(converted_back_slice, &slice);
//! # }
//! ```
//!
//! # Example with mutable slices
//! ```
//! # extern crate byte_slice_cast;
//! use byte_slice_cast::*;
//!
//! # fn main() {
//! let mut slice = [0u8; 4];
//! {
//!     let mut converted_slice = slice.as_mut_slice_of::<u32>().unwrap();
//!     converted_slice[0] = 0x12345678;
//! }
//!
//! if cfg!(target_endian = "big") {
//!     assert_eq!(&slice, &[0x12, 0x34, 0x56, 0x78]);
//! } else {
//!     assert_eq!(&slice, &[0x78, 0x56, 0x34, 0x12]);
//! }
//!
//! # }
//! ```

use std::slice;
use std::fmt;
use std::mem;

use std::error::Error as StdError;

/// Possible errors during slice conversion.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// The input slice is not properly aligned for the
    /// output data type. E.g. for an `u32` output slice
    /// the memory must be 4-byte aligned.
    WrongAlignment,
    /// A non-integer number of values from the output
    /// type would be in the output slice.
    IncompleteNumberOfValues,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str(self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        use self::Error::*;

        match *self {
            WrongAlignment => "Wrong Alignment",
            IncompleteNumberOfValues => "Incomplete Number of Values",
        }
    }
}

fn check_constraints<T, U>(data: &[T]) -> Result<usize, Error> {
    let alignment = mem::align_of::<U>();

    if (data.as_ptr() as usize) % alignment != 0 {
        return Err(Error::WrongAlignment);
    }

    let size_in = mem::size_of::<T>();
    let size_out = mem::size_of::<U>();
    if (data.len() * size_in) % size_out != 0 {
        return Err(Error::IncompleteNumberOfValues);
    }

    Ok((data.len() * size_in) / size_out)
}

/// Trait for converting from a byte slice to a slice of another fundamental, built-in numeric
/// type. Usually using the `AsSliceOf` trait is more convenient.
///
/// # Example
/// ```
/// # extern crate byte_slice_cast;
/// use byte_slice_cast::*;
///
/// # fn main() {
/// let slice = [1u8, 2u8, 3u8, 4u8, 5u8, 6u8];
/// let converted_slice = <u16 as FromByteSlice>::from_byte_slice(&slice).unwrap();
///
/// if cfg!(target_endian = "big") {
///     assert_eq!(converted_slice, &[0x0102, 0x0304, 0x0506]);
/// } else {
///     assert_eq!(converted_slice, &[0x0201, 0x0403, 0x0605]);
/// }
/// # }
/// ```
pub unsafe trait FromByteSlice
    where Self: Sized {
    /// Convert from an immutable byte slice to a immutable slice of another fundamental, built-in
    /// numeric type
    fn from_byte_slice<'a>(&'a [u8]) -> Result<&'a [Self], Error>;
    /// Convert from an mutable byte slice to a mutable slice of another fundamental, built-in
    /// numeric type
    fn from_mut_byte_slice<'a>(&'a mut [u8]) -> Result<&'a mut [Self], Error>;
}

/// Trait for converting from a slice of a fundamental, built-in numeric type to a byte slice.
///
/// # Example
/// ```
/// # extern crate byte_slice_cast;
/// use byte_slice_cast::*;
///
/// # fn main() {
/// let slice: [u16; 3] = [0x0102, 0x0304, 0x0506];
/// let converted_slice = slice.as_byte_slice().unwrap();
///
/// if cfg!(target_endian = "big") {
///     assert_eq!(converted_slice, &[1u8, 2u8, 3u8, 4u8, 5u8, 6u8]);
/// } else {
///     assert_eq!(converted_slice, &[2u8, 1u8, 4u8, 3u8, 6u8, 5u8]);
/// }
/// # }
/// ```
pub unsafe trait ToByteSlice {
    fn as_byte_slice<'a>(&'a self) -> Result<&'a [u8], Error>;
    fn as_mut_byte_slice<'a>(&'a mut self) -> Result<&'a mut [u8], Error>;
}

macro_rules! impl_trait(
    ($to:ty) => {
        unsafe impl FromByteSlice for $to {
            fn from_byte_slice<'a>(slice: &'a [u8]) -> Result<&'a [$to], Error> {
                let len = check_constraints::<u8, $to>(slice)?;
                unsafe {
                    Ok(slice::from_raw_parts(slice.as_ptr() as *const $to, len))
                }
            }

            fn from_mut_byte_slice<'a>(slice: &'a mut [u8]) -> Result<&'a mut [$to], Error> {
                let len = check_constraints::<u8, $to>(slice)?;
                unsafe {
                    Ok(slice::from_raw_parts_mut(slice.as_mut_ptr() as *mut $to, len))
                }
            }
        }

        unsafe impl ToByteSlice for [$to] {
            fn as_byte_slice<'a>(&'a self) -> Result<&'a [u8], Error> {
                let len = check_constraints::<$to, u8>(self)?;
                unsafe {
                    Ok(slice::from_raw_parts(self.as_ptr() as *const u8, len))
                }
            }

            fn as_mut_byte_slice<'a>(&'a mut self) -> Result<&'a mut [u8], Error> {
                let len = check_constraints::<$to, u8>(self)?;
                unsafe {
                    Ok(slice::from_raw_parts_mut(self.as_mut_ptr() as *mut u8, len))
                }
            }
        }
    };
);

impl_trait!(u8);
impl_trait!(u16);
impl_trait!(u32);
impl_trait!(u64);
impl_trait!(i8);
impl_trait!(i16);
impl_trait!(i32);
impl_trait!(i64);
impl_trait!(f32);
impl_trait!(f64);

/// Trait for converting from a byte slice to a slice of another fundamental, built-in numeric
/// type. This trait is usually more convenient to use than `FromByteSlice`.
///
/// # Example
/// ```
/// # extern crate byte_slice_cast;
/// use byte_slice_cast::*;
///
/// # fn main() {
/// let slice = [1u8, 2u8, 3u8, 4u8, 5u8, 6u8];
/// let converted_slice = slice.as_slice_of::<u16>().unwrap();
///
/// if cfg!(target_endian = "big") {
///     assert_eq!(converted_slice, &[0x0102, 0x0304, 0x0506]);
/// } else {
///     assert_eq!(converted_slice, &[0x0201, 0x0403, 0x0605]);
/// }
/// # }
/// ```
pub trait AsSliceOf {
    fn as_slice_of<'a, T: FromByteSlice>(&'a self) -> Result<&'a [T], Error>;
    fn as_mut_slice_of<'a, T: FromByteSlice>(&'a mut self) -> Result<&'a mut [T], Error>;
}

impl AsSliceOf for [u8] {
    fn as_slice_of<'a, T: FromByteSlice>(&'a self) -> Result<&'a [T], Error> {
        FromByteSlice::from_byte_slice(self)
    }

    fn as_mut_slice_of<'a, T: FromByteSlice>(&'a mut self) -> Result<&'a mut [T], Error> {
        FromByteSlice::from_mut_byte_slice(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8() {
        let input: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

        let output: &[u8] = input.as_slice_of::<u8>().unwrap();
        assert_eq!(&input, output);

        let output2: &[u8] = input.as_byte_slice().unwrap();
        assert_eq!(&input, output2);
    }

    #[test]
    fn u16() {
        let slice: [u16; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        let bytes = slice.as_byte_slice().unwrap();

        if cfg!(target_endian = "big") {
            assert_eq!(bytes, &[0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7]);
        } else {
            assert_eq!(bytes, &[0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0]);
        }


        assert_eq!(bytes[1..].as_slice_of::<u16>(), Err(Error::WrongAlignment));
        assert_eq!(
            [0u8; 17].as_slice_of::<u16>(),
            Err(Error::IncompleteNumberOfValues)
        );
        assert_eq!(bytes.as_slice_of::<u16>(), Ok(slice.as_ref()));
    }

    #[test]
    fn u32() {
        let slice: [u32; 4] = [0, 1, 2, 3];
        let bytes = slice.as_byte_slice().unwrap();

        if cfg!(target_endian = "big") {
            assert_eq!(bytes, &[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3]);
        } else {
            assert_eq!(bytes, &[0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
        }


        assert_eq!(bytes[1..].as_slice_of::<u32>(), Err(Error::WrongAlignment));
        assert_eq!(
            [0u8; 17].as_slice_of::<u32>(),
            Err(Error::IncompleteNumberOfValues)
        );
        assert_eq!(bytes.as_slice_of::<u32>(), Ok(slice.as_ref()));
    }

    #[test]
    fn u64() {
        let slice: [u64; 2] = [0, 1];
        let bytes = slice.as_byte_slice().unwrap();

        if cfg!(target_endian = "big") {
            assert_eq!(bytes, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        } else {
            assert_eq!(bytes, &[0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
        }


        assert_eq!(bytes[1..].as_slice_of::<u64>(), Err(Error::WrongAlignment));
        assert_eq!(
            [0u8; 17].as_slice_of::<u64>(),
            Err(Error::IncompleteNumberOfValues)
        );
        assert_eq!(bytes.as_slice_of::<u64>(), Ok(slice.as_ref()));
    }

    #[test]
    fn f32() {
        let slice: [f32; 4] = [2.0, 1.0, 0.5, 0.25];
        let bytes = slice.as_byte_slice().unwrap();

        if cfg!(target_endian = "big") {
            assert_eq!(
                bytes,
                [
                    0x40,
                    0x00,
                    0x00,
                    0x00,
                    0x3f,
                    0x80,
                    0x00,
                    0x00,
                    0x3f,
                    0x00,
                    0x00,
                    0x00,
                    0x3e,
                    0x80,
                    0x00,
                    0x00
                ]
            );
        } else {
            assert_eq!(
                bytes,
                [
                    0x00,
                    0x00,
                    0x00,
                    0x40,
                    0x00,
                    0x00,
                    0x80,
                    0x3f,
                    0x00,
                    0x00,
                    0x00,
                    0x3f,
                    0x00,
                    0x00,
                    0x80,
                    0x3e
                ]
            );
        };

        assert_eq!(bytes[1..].as_slice_of::<f32>(), Err(Error::WrongAlignment));
        assert_eq!(
            [0u8; 17].as_slice_of::<f32>(),
            Err(Error::IncompleteNumberOfValues)
        );
        assert_eq!(bytes.as_slice_of::<f32>(), Ok(slice.as_ref()));
    }


    #[test]
    fn f64() {
        let slice: [f64; 2] = [2.0, 0.5];
        let bytes = slice.as_byte_slice().unwrap();

        if cfg!(target_endian = "big") {
            assert_eq!(
                bytes,
                [
                    0x40,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x3f,
                    0xe0,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00
                ]
            );
        } else {
            assert_eq!(
                bytes,
                [
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x40,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0x00,
                    0xe0,
                    0x3f
                ]
            );
        };

        assert_eq!(bytes[1..].as_slice_of::<f64>(), Err(Error::WrongAlignment));
        assert_eq!(
            [0u8; 17].as_slice_of::<f64>(),
            Err(Error::IncompleteNumberOfValues)
        );
        assert_eq!(bytes.as_slice_of::<f64>(), Ok(slice.as_ref()));
    }
}
