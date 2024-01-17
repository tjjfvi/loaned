use crate::*;
use std::{
  fmt::{Debug, Display},
  hash::Hash,
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
pub struct Loaned<'t, T> {
  /// Invariant: the target of `inner` is borrowed for `'t`, so it may only be
  /// accessed immutably (not mutably or uniquely) for the duration of `'t`.
  pub(crate) inner: MaybeUninit<T>,
  // establish contravariance over `'t``
  pub(crate) __: PhantomData<fn(&'t ())>,
}

impl<'t, T> Loaned<'t, T> {
  /// Constructs a `Loaned` from a given smart pointer, returning the borrow
  /// along with the loaned pointer.
  #[inline]
  pub fn loan(value: T) -> (&'t T::Target, Self)
  where
    T: Loanable<'t>,
  {
    let loaned = unsafe { Loaned::from_inner(MaybeUninit::new(value)) };
    (loaned.borrow(), loaned)
  }

  /// Creates a `Loaned` without actually loaning it. If you want to loan it,
  /// use `Loaned::loan` or `Loaned::borrow`.
  #[inline(always)]
  pub fn new(value: T) -> Self {
    unsafe { Loaned::from_inner(MaybeUninit::new(value)) }
  }

  #[inline(always)]
  pub fn place(self, place: &'t mut impl Place<'t, T>) {
    place.place(self.into())
  }

  #[inline(always)]
  pub fn borrow(&self) -> &'t T::Target
  where
    T: Loanable<'t>,
  {
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

impl<'t, T> Deref for Loaned<'t, T> {
  type Target = T;
  #[inline(always)]
  fn deref(&self) -> &T {
    unsafe { self.inner.assume_init_ref() }
  }
}

impl<'t, T> Drop for Loaned<'t, T> {
  #[cold]
  fn drop(&mut self) {
    if std::mem::needs_drop::<T>() && !std::thread::panicking() {
      panic!(
        "memory leak: cannot drop `{Self}`
    if leaking is desired, use `ManuallyDrop<{Self}>` or `mem::forget`
    otherwise, use `loaned.place(&mut None)` to drop the inner value",
        Self = std::any::type_name::<Self>()
      )
    }
  }
}

impl<'t, T: Debug> Debug for Loaned<'t, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Loaned").field(&*self).finish()
  }
}

impl<'t, T: Display> Display for Loaned<'t, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    (&**self).fmt(f)
  }
}

impl<'t, T: Clone> Clone for Loaned<'t, T> {
  fn clone(&self) -> Self {
    Loaned::new((&**self).clone())
  }
}

impl<'t, T: Default> Default for Loaned<'t, T> {
  fn default() -> Self {
    Loaned::new(Default::default())
  }
}

impl<'t, 'u, T: PartialEq<U>, U> PartialEq<Loaned<'u, U>> for Loaned<'t, T> {
  fn eq(&self, other: &Loaned<'u, U>) -> bool {
    (&**self) == (&**other)
  }
}

impl<'t, T: Eq> Eq for Loaned<'t, T> {}

impl<'t, 'u, T: PartialOrd<U>, U> PartialOrd<Loaned<'u, U>> for Loaned<'t, T> {
  fn partial_cmp(&self, other: &Loaned<'u, U>) -> Option<std::cmp::Ordering> {
    (&**self).partial_cmp(&**other)
  }
}

impl<'t, T: Ord> Ord for Loaned<'t, T> {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    (&**self).cmp(&**other)
  }
}

impl<'t, T: Hash> Hash for Loaned<'t, T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    (&**self).hash(state);
  }
}

impl<'t, T> From<T> for Loaned<'t, T> {
  fn from(value: T) -> Self {
    Loaned::new(value)
  }
}
