use crate::*;
use std::{
  fmt::Debug,
  marker::PhantomData,
  mem::{ManuallyDrop, MaybeUninit},
  ops::Deref,
};

/// `Loaned<'t, T>` connotes ownership of a smart pointer `T`, with the caveat
/// that its target is immutably loaned for `'t` (i.e. something else may hold
/// an `&'t` reference to the target of this pointer).
///
/// Thus, for the duration of `'t`, one cannot mutably access the target of this
/// pointer. However, unlike `LoanedMut`, one can immutably access the
/// target.
///
/// Like `LoanedMut`, one can store this smart pointer somewhere, as long
/// as its ensured that it won't be used for the duration of `'t`.
///
/// This can be done with `Loaned::place`, which can store the smart pointer
/// into an `&'t mut T` (among other things).
///
/// # Dropping
///
/// The smart pointer held by a `Loaned` can only be dropped once `'t` expires.
/// Since there is no way in the type system to enforce this, nor any way to
/// check this at runtime, dropping a `Loaned` panics.
///
/// If leaking is intentional, use a `ManuallyDrop<LoanedMut<'t, T>>`.
///
/// Otherwise, use `loaned.place(&mut None)` to drop the inner value.
#[must_use]
#[repr(transparent)]
pub struct Loaned<'t, T: Loanable<'t>> {
  /// Invariant: the target of `inner` is borrowed for `'t`, so it may only be
  /// accessed immutably (not mutably or uniquely) for the duration of `'t`.
  pub(crate) inner: MaybeUninit<T>,
  // establish contravariance over `'t``
  pub(crate) __: PhantomData<fn(&'t ())>,
}

impl<'t, T: Loanable<'t>> Loaned<'t, T> {
  /// Constructs a `Loaned` from a given smart pointer, returning the borrow
  /// along with the loaned pointer.
  #[inline]
  pub fn new(value: T) -> (&'t T::Target, Self) {
    let loaned = unsafe { Loaned::from_inner(MaybeUninit::new(value)) };
    (loaned.borrow(), loaned)
  }

  #[inline(always)]
  pub fn place(self, place: &'t mut impl Place<'t, T>) {
    place.place(self.into())
  }

  #[inline(always)]
  pub fn borrow(&self) -> &'t T::Target {
    unsafe { &*(&**self.inner.assume_init_ref() as *const _) }
  }

  #[inline(always)]
  pub(crate) fn into_inner(self) -> MaybeUninit<T> {
    unsafe { std::ptr::read(&ManuallyDrop::new(self).inner) }
  }

  #[inline(always)]
  pub(crate) unsafe fn from_inner(inner: MaybeUninit<T>) -> Self {
    Loaned {
      inner,
      __: PhantomData,
    }
  }
}

impl<'t, T: Loanable<'t>> Deref for Loaned<'t, T> {
  type Target = T::Target;
  #[inline(always)]
  fn deref(&self) -> &T::Target {
    unsafe { &*(&**self.inner.assume_init_ref() as *const _) }
  }
}

impl<'t, T: Loanable<'t>> Drop for Loaned<'t, T> {
  #[cold]
  fn drop(&mut self) {
    if T::NEEDS_DROP && !std::thread::panicking() {
      panic!(
        "memory leak: cannot drop `{Self}`
    if leaking is desired, use `ManuallyDrop<{Self}>` or `mem::forget`
    otherwise, use `loaned.place(&mut None)` to drop the inner value",
        Self = std::any::type_name::<Self>()
      )
    }
  }
}

impl<'t, T: Loanable<'t>> Debug for Loaned<'t, T>
where
  T::Target: Debug + Sized,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Loaned").field(&**self).finish()
  }
}
