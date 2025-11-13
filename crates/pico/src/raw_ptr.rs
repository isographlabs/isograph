use core::ptr::{self, NonNull};

#[repr(transparent)]
pub struct RawPtr<T: ?Sized> {
    ptr: NonNull<T>,
}

impl<T: ?Sized> Copy for RawPtr<T> {}
impl<T: ?Sized> Clone for RawPtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> PartialEq for RawPtr<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.ptr.as_ptr(), other.ptr.as_ptr())
    }
}

impl<T: ?Sized> Eq for RawPtr<T> {}

impl<T: ?Sized> RawPtr<T> {
    #[inline]
    pub fn from_ref(r: &T) -> Self {
        Self {
            ptr: NonNull::from(r),
        }
    }

    /// # Safety
    /// Caller must guarantee that:
    /// - `self.ptr` points to a valid `T` that lives for at least `'a`.
    /// - The pointee will not be moved while the returned
    ///   reference is alive.
    /// - The pointer was originally derived from a valid `&T`.
    #[inline]
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        unsafe {
            // SAFETY: the above preconditions ensure that `self.ptr` is non-null,
            // properly aligned, and points to a live `T` for `'a`.
            self.ptr.as_ref()
        }
    }
}
