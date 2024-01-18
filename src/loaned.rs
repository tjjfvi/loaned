use crate::*;
use std::{
  fmt::{Debug, Display},
  hash::Hash,
  marker::PhantomData,
  mem::{ManuallyDrop, MaybeUninit},
  ops::Deref,
};

/// `Loaned<'t, T>` connotes ownership of a value `T`, with the caveat that
/// allocations owned by it are immutably loaned for `'t` (i.e. something else
/// may hold an `&'t` reference to such allocations).
///
/// Thus, for the duration of `'t`, one cannot mutably access this value.
/// However, unlike `LoanedMut`, one can immutably access it.
///
/// One can store this value somewhere with `Loaned::place`, which will ensure
/// that it cannot be used for the duration of `'t`.
///
/// Taking the value out of a `LoanedMut` can be done with the `take!` macro,
/// which will statically ensure that `'t` has expired.
///
/// # Dropping
///
/// The value held by a `Loaned` can only be dropped once `'t` expires. Since
/// there is no way in the type system to enforce this, nor any way to check
/// this at runtime, dropping a `Loaned` panics.
///
/// If leaking is intentional, use a `ManuallyDrop<LoanedMut<'t, T>>`.
///
/// To drop the inner value, use `drop!(loaned)`, which will statically ensure
/// that `'t` has expired.
#[must_use = "dropping a `Loaned` panics; use `loaned::drop!` instead"]
#[repr(transparent)]
pub struct Loaned<'t, T> {
  /// Invariant: the target of `inner` is borrowed for `'t`, so it may only be
  /// accessed immutably (not mutably or uniquely) for the duration of `'t`.
  pub(crate) inner: MaybeUninit<T>,
  pub(crate) _contravariant: PhantomData<fn(&'t ())>,
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

  /// Stores the contained value into a given place. See the `Place` trait for
  /// more.
  #[inline(always)]
  pub fn place(self, place: &'t mut impl Place<'t, T>) {
    place.place(self.into())
  }

  /// Borrows the pointee of the value, returning a reference valid for `'t`.
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
      _contravariant: PhantomData,
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
    otherwise, use `drop!(loaned)` to drop the inner value",
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

impl<'t, T> Loaned<'t, T> {
  /// Merges multiple `LoanedMut` values.
  ///
  /// # Example
  /// ```
  /// use loaned::Loaned;
  /// let a = Loaned::new(1);
  /// let b = Loaned::new(2);
  /// let ab: Loaned<(u32, u32)> = Loaned::merge(Default::default(), |ab, m| {
  ///   m.place(a, &mut ab.0);
  ///   m.place(b, &mut ab.1);
  /// });
  /// ```
  pub fn merge(value: T, f: impl for<'i> FnOnce(&'i mut T, &'i Merge<'t, 'i>)) -> Self {
    unsafe {
      let mut inner = MaybeUninit::new(value);
      f(inner.assume_init_mut(), &Merge(PhantomData));
      Loaned::from_inner(inner)
    }
  }
}

/// See `Loaned::merge`.
#[doc(hidden)]
pub struct Merge<'t, 'i>(PhantomData<(&'t mut &'t (), &'i mut &'i ())>);

impl<'t, 'i> Merge<'t, 'i> {
  /// See `Loaned::merge`.
  pub fn place<T>(&'i self, loaned: Loaned<'t, T>, place: &'i mut impl Place<'i, T>) {
    place.place(unsafe { LoanedMut::from_inner(loaned.into_inner()) })
  }
}

impl<'t, T> From<Box<Loaned<'t, T>>> for Loaned<'t, Box<T>> {
  fn from(value: Box<Loaned<'t, T>>) -> Self {
    unsafe { Loaned::new(Box::from_raw(Box::into_raw(value) as *mut _)) }
  }
}

impl<'t, T> From<Vec<Loaned<'t, T>>> for Loaned<'t, Vec<T>> {
  fn from(value: Vec<Loaned<'t, T>>) -> Self {
    unsafe {
      let mut value = ManuallyDrop::new(value);
      Loaned::new(Vec::from_raw_parts(
        value.as_mut_ptr() as *mut _,
        value.len(),
        value.capacity(),
      ))
    }
  }
}

impl<'t, T> Loaned<'t, T> {
  /// Creates a `Loaned` with multiple sub-loans.
  ///
  /// # Example
  /// ```
  /// use loaned::Loaned;
  /// let ((a, b), ab) = Loaned::loan_with((Box::new(1), Box::new(2)), |ab, l| {
  ///   (l.loan(&ab.0), l.loan(&ab.1))
  /// });
  /// assert_eq!(*a, 1);
  /// assert_eq!(*b, 2);
  /// assert_eq!(loaned::take!(ab), (Box::new(1), Box::new(2)));
  /// ```
  pub fn loan_with<L>(
    value: T,
    f: impl for<'i> FnOnce(&'i mut T, &'i LoanWith<'t, 'i>) -> L,
  ) -> (L, Self) {
    unsafe {
      let mut inner = MaybeUninit::new(value);
      let loans = f(inner.assume_init_mut(), &LoanWith(PhantomData));
      (loans, Loaned::from_inner(inner))
    }
  }
}

/// See `Loaned::loan_with`.
#[doc(hidden)]
pub struct LoanWith<'t, 'i>(PhantomData<(&'t mut &'t (), &'i mut &'i ())>);

impl<'t, 'i> LoanWith<'t, 'i> {
  /// See `Loaned::loan_with`.
  pub fn loan<T: Loanable<'i>>(&'i self, value: &'i T) -> &'t T::Target {
    unsafe { &*(&**value as *const _) }
  }
}
