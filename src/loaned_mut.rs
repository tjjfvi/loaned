use crate::*;
use core::fmt::Debug;

/// `LoanedMut<'t, T>` connotes ownership of a value `T`, with the caveat that
/// allocations owned by it are mutably loaned for `'t` (i.e. something else may
/// hold an `&'t mut` reference to such allocations).
///
/// Thus, for the duration of `'t`, one cannot access this value.
///
/// One can, however, store this value somewhere with [`LoanedMut::place`],
/// which will ensure that it cannot be used for the duration of `'t`.
///
/// Taking the value out of a `LoanedMut` can be done with the [`take!`] macro,
/// which will statically ensure that `'t` has expired.
///
/// # Dropping
///
/// The value held by a `LoanedMut` can only be dropped once `'t` expires. Since
/// there is no way in the type system to enforce this, nor any way to check
/// this at runtime, dropping a `LoanedMut` panics.
///
/// If leaking is intentional, use a `ManuallyDrop<LoanedMut<'t, T>>`.
///
/// To drop the inner value, use the [`drop!`] macro, which will statically ensure
/// that `'t` has expired.
#[must_use = "dropping a `LoanedMut` panics; use `loaned::drop!` instead"]
#[repr(transparent)]
pub struct LoanedMut<'t, T> {
  /// Invariant: the target of `inner` is mutably borrowed for `'t`, so it may
  /// not be accessed for the duration of `'t`.
  pub(crate) inner: RawLoaned<T>,
  pub(crate) _contravariant: PhantomData<fn(&'t ())>,
}

impl<'t, T> LoanedMut<'t, T> {
  /// Constructs a `LoanedMut` from a given smart pointer, returning the mutable
  /// borrow along with the loaned pointer.
  #[inline]
  pub fn loan(value: T) -> (&'t mut T::Target, Self)
  where
    T: Loanable<'t> + DerefMut,
  {
    let mut inner = RawLoaned::new(value);
    let borrow = unsafe { &mut *(&mut **inner.as_mut() as *mut _) };
    (borrow, unsafe { LoanedMut::from_raw(inner) })
  }

  /// Creates a `LoanedMut` without actually loaning it. If you want to loan it,
  /// use [`LoanedMut::loan`].
  #[inline(always)]
  pub fn new(value: T) -> Self {
    unsafe { LoanedMut::from_raw(RawLoaned::new(value)) }
  }

  /// Stores the contained value into a given place. See the [`Place`] trait for
  /// more.
  #[inline(always)]
  pub fn place(self, place: &'t mut impl Place<'t, T>) {
    Place::place(self, place)
  }

  #[inline(always)]
  pub(crate) fn into_raw(self) -> RawLoaned<T> {
    unsafe { ptr::read(&ManuallyDrop::new(self).inner) }
  }

  #[inline(always)]
  pub(crate) unsafe fn from_raw(inner: RawLoaned<T>) -> Self {
    LoanedMut {
      inner,
      _contravariant: PhantomData,
    }
  }
}

impl<'t, T> From<Loaned<'t, T>> for LoanedMut<'t, T> {
  #[inline(always)]
  fn from(value: Loaned<'t, T>) -> Self {
    unsafe { LoanedMut::from_raw(value.into_raw()) }
  }
}

impl<'t, T> Drop for LoanedMut<'t, T> {
  #[cold]
  fn drop(&mut self) {
    #[cfg(feature = "std")]
    if mem::needs_drop::<T>() && !std::thread::panicking() {
      panic!(
        "memory leak: cannot drop `{Self}`
    if leaking is desired, use `ManuallyDrop<{Self}>` or `mem::forget`
    otherwise, use `drop!(loaned)` to drop the inner value",
        Self = core::any::type_name::<Self>()
      )
    }
  }
}

impl<'t, T> Debug for LoanedMut<'t, T> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "LoanedMut(..)")
  }
}

impl<'t, T: Default> Default for LoanedMut<'t, T> {
  fn default() -> Self {
    LoanedMut::new(Default::default())
  }
}

impl<'t, T> From<T> for LoanedMut<'t, T> {
  fn from(value: T) -> Self {
    LoanedMut::new(value)
  }
}

impl<'t, T> LoanedMut<'t, T> {
  /// Merges multiple `LoanedMut` values.
  ///
  /// # Example
  /// ```
  /// use loaned::LoanedMut;
  /// let a = LoanedMut::new(1);
  /// let b = LoanedMut::new(2);
  /// let ab: LoanedMut<(u32, u32)> = LoanedMut::merge(Default::default(), |ab, m| {
  ///   m.place(a, &mut ab.0);
  ///   m.place(b, &mut ab.1);
  /// });
  /// ```
  pub fn merge(value: T, f: impl for<'i> FnOnce(&'i mut T, &'i MergeMut<'t, 'i>)) -> Self {
    unsafe {
      let mut inner = RawLoaned::new(value);
      f(inner.as_mut(), &MergeMut(PhantomData));
      LoanedMut::from_raw(inner)
    }
  }
}

/// See [`LoanedMut::merge`].
#[doc(hidden)]
pub struct MergeMut<'t, 'i>(PhantomData<(&'t mut &'t (), &'i mut &'i ())>);

impl<'t, 'i> MergeMut<'t, 'i> {
  /// See [`LoanedMut::merge`].
  pub fn place<T>(&'i self, loaned: LoanedMut<'t, T>, place: &'i mut impl Place<'i, T>) {
    Place::place(unsafe { LoanedMut::from_raw(loaned.into_raw()) }, place)
  }
}

impl<'t, T> LoanedMut<'t, T> {
  /// Creates a `LoanedMut` with multiple sub-loans.
  ///
  /// # Example
  /// ```
  /// use loaned::LoanedMut;
  /// let ((a, b), ab) = LoanedMut::loan_with((Box::new(0), Box::new(0)), |ab, l| {
  ///   (l.loan_mut(&mut ab.0), l.loan_mut(&mut ab.1))
  /// });
  /// *a = 1;
  /// *b = 2;
  /// assert_eq!(loaned::take!(ab), (Box::new(1), Box::new(2)));
  /// ```
  pub fn loan_with<L>(
    value: T,
    f: impl for<'i> FnOnce(&'i mut T, &'i LoanWithMut<'t, 'i>) -> L,
  ) -> (L, Self) {
    unsafe {
      let mut inner = RawLoaned::new(value);
      let loans = f(inner.as_mut(), &LoanWithMut(PhantomData));
      (loans, LoanedMut::from_raw(inner))
    }
  }
}

/// See [`LoanedMut::loan_with`].
#[doc(hidden)]
pub struct LoanWithMut<'t, 'i>(PhantomData<(&'t mut &'t (), &'i mut &'i ())>);

impl<'t, 'i> LoanWithMut<'t, 'i> {
  /// See [`LoanedMut::loan_with`].
  pub fn loan_mut<T: Loanable<'i> + DerefMut>(&'i self, value: &'i mut T) -> &'t mut T::Target {
    unsafe { &mut *(&mut **value as *mut _) }
  }
  /// See [`LoanedMut::loan_with`].
  pub fn loan<T: Loanable<'i>>(&'i self, value: &'i T) -> &'t T::Target {
    unsafe { &*(&**value as *const _) }
  }
}
