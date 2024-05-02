use num_complex::Complex;
use std::mem;
use std::ptr::NonNull;

use crate::dimension::{self, stride_offset};
use crate::extension::nonnull::nonnull_debug_checked_from_ptr;
use crate::imp_prelude::*;
use crate::is_aligned;
use crate::shape_builder::{Strides, StrideShape};

impl<A, D> RawArrayView<A, D>
where
    D: Dimension,
{
    /// Create a new `RawArrayView`.
    ///
    /// Unsafe because caller is responsible for ensuring that the array will
    /// meet all of the invariants of the `ArrayBase` type.
    #[inline]
    pub(crate) unsafe fn new(ptr: NonNull<A>, dim: D, strides: D) -> Self {
        RawArrayView::from_data_ptr(RawViewRepr::new(), ptr)
            .with_strides_dim(strides, dim)
    }

    unsafe fn new_(ptr: *const A, dim: D, strides: D) -> Self {
        Self::new(nonnull_debug_checked_from_ptr(ptr as *mut A), dim, strides)
    }

    /// Create an `RawArrayView<A, D>` from shape information and a raw pointer
    /// to the elements.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring all of the following:
    ///
    /// * `ptr` must be non-null, and it must be safe to [`.offset()`] `ptr` by
    ///   zero.
    ///
    /// * It must be safe to [`.offset()`] the pointer repeatedly along all
    ///   axes and calculate the `count`s for the `.offset()` calls without
    ///   overflow, even if the array is empty or the elements are zero-sized.
    ///
    ///   In other words,
    ///
    ///   * All possible pointers generated by moving along all axes must be in
    ///     bounds or one byte past the end of a single allocation with element
    ///     type `A`. The only exceptions are if the array is empty or the element
    ///     type is zero-sized. In these cases, `ptr` may be dangling, but it must
    ///     still be safe to [`.offset()`] the pointer along the axes.
    ///
    ///   * The offset in units of bytes between the least address and greatest
    ///     address by moving along all axes must not exceed `isize::MAX`. This
    ///     constraint prevents the computed offset, in bytes, from overflowing
    ///     `isize` regardless of the starting point due to past offsets.
    ///
    ///   * The offset in units of `A` between the least address and greatest
    ///     address by moving along all axes must not exceed `isize::MAX`. This
    ///     constraint prevents overflow when calculating the `count` parameter to
    ///     [`.offset()`] regardless of the starting point due to past offsets.
    ///
    /// * The product of non-zero axis lengths must not exceed `isize::MAX`.
    /// 
    /// * Strides must be non-negative.
    ///
    /// This function can use debug assertions to check some of these requirements,
    /// but it's not a complete check.
    ///
    /// [`.offset()`]: https://doc.rust-lang.org/stable/std/primitive.pointer.html#method.offset
    pub unsafe fn from_shape_ptr<Sh>(shape: Sh, ptr: *const A) -> Self
    where
        Sh: Into<StrideShape<D>>,
    {
        let shape = shape.into();
        let dim = shape.dim;
        if cfg!(debug_assertions) {
            assert!(!ptr.is_null(), "The pointer must be non-null.");
            if let Strides::Custom(strides) = &shape.strides {
                dimension::strides_non_negative(strides).unwrap();
                dimension::max_abs_offset_check_overflow::<A, _>(&dim, strides).unwrap();
            } else {
                dimension::size_of_shape_checked(&dim).unwrap();
            }
        }
        let strides = shape.strides.strides_for_dim(&dim);
        RawArrayView::new_(ptr, dim, strides)
    }

    /// Converts to a read-only view of the array.
    ///
    /// # Safety
    ///
    /// From a safety standpoint, this is equivalent to dereferencing a raw
    /// pointer for every element in the array. You must ensure that all of the
    /// data is valid, ensure that the pointer is aligned, and choose the
    /// correct lifetime.
    #[inline]
    pub unsafe fn deref_into_view<'a>(self) -> ArrayView<'a, A, D> {
        debug_assert!(
            is_aligned(self.ptr.as_ptr()),
            "The pointer must be aligned."
        );
        ArrayView::new(self.ptr, self.dim, self.strides)
    }

    /// Split the array view along `axis` and return one array pointer strictly
    /// before the split and one array pointer after the split.
    ///
    /// **Panics** if `axis` or `index` is out of bounds.
    pub fn split_at(self, axis: Axis, index: Ix) -> (Self, Self) {
        assert!(index <= self.len_of(axis));
        let left_ptr = self.ptr.as_ptr();
        let right_ptr = if index == self.len_of(axis) {
            self.ptr.as_ptr()
        } else {
            let offset = stride_offset(index, self.strides.axis(axis));
            // The `.offset()` is safe due to the guarantees of `RawData`.
            unsafe { self.ptr.as_ptr().offset(offset) }
        };

        let mut dim_left = self.dim.clone();
        dim_left.set_axis(axis, index);
        let left = unsafe { Self::new_(left_ptr, dim_left, self.strides.clone()) };

        let mut dim_right = self.dim;
        let right_len = dim_right.axis(axis) - index;
        dim_right.set_axis(axis, right_len);
        let right = unsafe { Self::new_(right_ptr, dim_right, self.strides) };

        (left, right)
    }

    /// Cast the raw pointer of the raw array view to a different type
    ///
    /// **Panics** if element size is not compatible.
    ///
    /// Lack of panic does not imply it is a valid cast. The cast works the same
    /// way as regular raw pointer casts.
    ///
    /// While this method is safe, for the same reason as regular raw pointer
    /// casts are safe, access through the produced raw view is only possible
    /// in an unsafe block or function.
    pub fn cast<B>(self) -> RawArrayView<B, D> {
        assert_eq!(
            mem::size_of::<B>(),
            mem::size_of::<A>(),
            "size mismatch in raw view cast"
        );
        let ptr = self.ptr.cast::<B>();
        unsafe { RawArrayView::new(ptr, self.dim, self.strides) }
    }
}

impl<T, D> RawArrayView<Complex<T>, D>
where
    D: Dimension,
{
    /// Splits the view into views of the real and imaginary components of the
    /// elements.
    pub fn split_complex(self) -> Complex<RawArrayView<T, D>> {
        // Check that the size and alignment of `Complex<T>` are as expected.
        // These assertions should always pass, for arbitrary `T`.
        assert_eq!(
            mem::size_of::<Complex<T>>(),
            mem::size_of::<T>().checked_mul(2).unwrap()
        );
        assert_eq!(mem::align_of::<Complex<T>>(), mem::align_of::<T>());

        let dim = self.dim.clone();

        // Double the strides. In the zero-sized element case and for axes of
        // length <= 1, we leave the strides as-is to avoid possible overflow.
        let mut strides = self.strides.clone();
        if mem::size_of::<T>() != 0 {
            for ax in 0..strides.ndim() {
                if dim[ax] > 1 {
                    strides[ax] = (strides[ax] as isize * 2) as usize;
                }
            }
        }

        let ptr_re: *mut T = self.ptr.as_ptr().cast();
        let ptr_im: *mut T = if self.is_empty() {
            // In the empty case, we can just reuse the existing pointer since
            // it won't be dereferenced anyway. It is not safe to offset by
            // one, since the allocation may be empty.
            ptr_re
        } else {
            // In the nonempty case, we can safely offset into the first
            // (complex) element.
            unsafe { ptr_re.add(1) }
        };

        // `Complex` is `repr(C)` with only fields `re: T` and `im: T`. So, the
        // real components of the elements start at the same pointer, and the
        // imaginary components start at the pointer offset by one, with
        // exactly double the strides. The new, doubled strides still meet the
        // overflow constraints:
        //
        // - For the zero-sized element case, the strides are unchanged in
        //   units of bytes and in units of the element type.
        //
        // - For the nonzero-sized element case:
        //
        //   - In units of bytes, the strides are unchanged. The only exception
        //     is axes of length <= 1, but those strides are irrelevant anyway.
        //
        //   - Since `Complex<T>` for nonzero `T` is always at least 2 bytes,
        //     and the original strides did not overflow in units of bytes, we
        //     know that the new, doubled strides will not overflow in units of
        //     `T`.
        unsafe {
            Complex {
                re: RawArrayView::new_(ptr_re, dim.clone(), strides.clone()),
                im: RawArrayView::new_(ptr_im, dim, strides),
            }
        }
    }
}

impl<A, D> RawArrayViewMut<A, D>
where
    D: Dimension,
{
    /// Create a new `RawArrayViewMut`.
    ///
    /// Unsafe because caller is responsible for ensuring that the array will
    /// meet all of the invariants of the `ArrayBase` type.
    #[inline]
    pub(crate) unsafe fn new(ptr: NonNull<A>, dim: D, strides: D) -> Self {
        RawArrayViewMut::from_data_ptr(RawViewRepr::new(), ptr)
            .with_strides_dim(strides, dim)
    }

    unsafe fn new_(ptr: *mut A, dim: D, strides: D) -> Self {
        Self::new(nonnull_debug_checked_from_ptr(ptr), dim, strides)
    }

    /// Create an `RawArrayViewMut<A, D>` from shape information and a raw
    /// pointer to the elements.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring all of the following:
    ///
    /// * `ptr` must be non-null, and it must be safe to [`.offset()`] `ptr` by
    ///   zero.
    ///
    /// * It must be safe to [`.offset()`] the pointer repeatedly along all
    ///   axes and calculate the `count`s for the `.offset()` calls without
    ///   overflow, even if the array is empty or the elements are zero-sized.
    ///
    ///   In other words,
    ///
    ///   * All possible pointers generated by moving along all axes must be in
    ///     bounds or one byte past the end of a single allocation with element
    ///     type `A`. The only exceptions are if the array is empty or the element
    ///     type is zero-sized. In these cases, `ptr` may be dangling, but it must
    ///     still be safe to [`.offset()`] the pointer along the axes.
    ///
    ///   * The offset in units of bytes between the least address and greatest
    ///     address by moving along all axes must not exceed `isize::MAX`. This
    ///     constraint prevents the computed offset, in bytes, from overflowing
    ///     `isize` regardless of the starting point due to past offsets.
    ///
    ///   * The offset in units of `A` between the least address and greatest
    ///     address by moving along all axes must not exceed `isize::MAX`. This
    ///     constraint prevents overflow when calculating the `count` parameter to
    ///     [`.offset()`] regardless of the starting point due to past offsets.
    ///
    /// * The product of non-zero axis lengths must not exceed `isize::MAX`.
    /// 
    /// * Strides must be non-negative.
    ///
    /// This function can use debug assertions to check some of these requirements,
    /// but it's not a complete check.
    ///
    /// [`.offset()`]: https://doc.rust-lang.org/stable/std/primitive.pointer.html#method.offset
    pub unsafe fn from_shape_ptr<Sh>(shape: Sh, ptr: *mut A) -> Self
    where
        Sh: Into<StrideShape<D>>,
    {
        let shape = shape.into();
        let dim = shape.dim;
        if cfg!(debug_assertions) {
            assert!(!ptr.is_null(), "The pointer must be non-null.");
            if let Strides::Custom(strides) = &shape.strides {
                dimension::strides_non_negative(strides).unwrap();
                dimension::max_abs_offset_check_overflow::<A, _>(&dim, strides).unwrap();
            } else {
                dimension::size_of_shape_checked(&dim).unwrap();
            }
        }
        let strides = shape.strides.strides_for_dim(&dim);
        RawArrayViewMut::new_(ptr, dim, strides)
    }

    /// Converts to a non-mutable `RawArrayView`.
    #[inline]
    pub(crate) fn into_raw_view(self) -> RawArrayView<A, D> {
        unsafe { RawArrayView::new(self.ptr, self.dim, self.strides) }
    }

    /// Converts to a read-only view of the array.
    ///
    /// # Safety
    ///
    /// From a safety standpoint, this is equivalent to dereferencing a raw
    /// pointer for every element in the array. You must ensure that all of the
    /// data is valid, ensure that the pointer is aligned, and choose the
    /// correct lifetime.
    #[inline]
    pub unsafe fn deref_into_view<'a>(self) -> ArrayView<'a, A, D> {
        debug_assert!(
            is_aligned(self.ptr.as_ptr()),
            "The pointer must be aligned."
        );
        ArrayView::new(self.ptr, self.dim, self.strides)
    }

    /// Converts to a mutable view of the array.
    ///
    /// # Safety
    ///
    /// From a safety standpoint, this is equivalent to dereferencing a raw
    /// pointer for every element in the array. You must ensure that all of the
    /// data is valid, ensure that the pointer is aligned, and choose the
    /// correct lifetime.
    #[inline]
    pub unsafe fn deref_into_view_mut<'a>(self) -> ArrayViewMut<'a, A, D> {
        debug_assert!(
            is_aligned(self.ptr.as_ptr()),
            "The pointer must be aligned."
        );
        ArrayViewMut::new(self.ptr, self.dim, self.strides)
    }

    /// Split the array view along `axis` and return one array pointer strictly
    /// before the split and one array pointer after the split.
    ///
    /// **Panics** if `axis` or `index` is out of bounds.
    pub fn split_at(self, axis: Axis, index: Ix) -> (Self, Self) {
        let (left, right) = self.into_raw_view().split_at(axis, index);
        unsafe {
            (
                Self::new(left.ptr, left.dim, left.strides),
                Self::new(right.ptr, right.dim, right.strides),
            )
        }
    }

    /// Cast the raw pointer of the raw array view to a different type
    ///
    /// **Panics** if element size is not compatible.
    ///
    /// Lack of panic does not imply it is a valid cast. The cast works the same
    /// way as regular raw pointer casts.
    ///
    /// While this method is safe, for the same reason as regular raw pointer
    /// casts are safe, access through the produced raw view is only possible
    /// in an unsafe block or function.
    pub fn cast<B>(self) -> RawArrayViewMut<B, D> {
        assert_eq!(
            mem::size_of::<B>(),
            mem::size_of::<A>(),
            "size mismatch in raw view cast"
        );
        let ptr = self.ptr.cast::<B>();
        unsafe { RawArrayViewMut::new(ptr, self.dim, self.strides) }
    }
}

impl<T, D> RawArrayViewMut<Complex<T>, D>
where
    D: Dimension,
{
    /// Splits the view into views of the real and imaginary components of the
    /// elements.
    pub fn split_complex(self) -> Complex<RawArrayViewMut<T, D>> {
        let Complex { re, im } = self.into_raw_view().split_complex();
        unsafe {
            Complex {
                re: RawArrayViewMut::new(re.ptr, re.dim, re.strides),
                im: RawArrayViewMut::new(im.ptr, im.dim, im.strides),
            }
        }
    }
}