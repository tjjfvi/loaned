use crate::*;
use std::{
  fmt::Debug,
  marker::PhantomData,
  mem::{ManuallyDrop, MaybeUninit},
  ops::DerefMut,
};

/// `LoanedMut<'t, T>` connotes ownership of a value `T`, with the caveat that
/// allocations owned by it are mutably loaned for `'t` (i.e. something else may
/// hold an `&'t mut` reference to such allocations).
///
/// Thus, for the duration of `'t`, one cannot access this value.
///
/// One can, however, store this value somewhere with `LoanedMut::place`, which
/// will ensure that it cannot be used for the duration of `'t`.
///
/// Taking the value out of a `Loaned` can be done with the `take!` macro, which
/// will statically ensure that `'t` has expired.
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
#[must_use = "dropping a `LoanedMut` panics; use `loaned::drop!` instead"]
#[repr(transparent)]
pub struct LoanedMut<'t, T> {
  /// Invariant: the target of `inner` is mutably borrowed for `'t`, so it may
  /// not be accessed for the duration of `'t`.
  pub(crate) inner: MaybeUninit<T>,
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
    let mut inner = MaybeUninit::new(value);
    let borrow = unsafe { &mut *(&mut **inner.assume_init_mut() as *mut _) };
    (borrow, unsafe { LoanedMut::from_inner(inner) })
  }

  /// Creates a `LoanedMut` without actually loaning it. If you want to loan it,
  /// use `LoanedMut::loan`.
  #[inline(always)]
  pub fn new(value: T) -> Self {
    unsafe { LoanedMut::from_inner(MaybeUninit::new(value)) }
  }

  /// Stores the contained value into a given place. See the `Place` trait for
  /// more.
  #[inline(always)]
  pub fn place(self, place: &'t mut impl Place<'t, T>) {
    place.place(self)
  }

  #[inline(always)]
  pub(crate) fn into_inner(self) -> MaybeUninit<T> {
    unsafe { std::ptr::read(&ManuallyDrop::new(self).inner) }
  }

  #[inline(always)]
  pub(crate) unsafe fn from_inner(inner: MaybeUninit<T>) -> Self {
    LoanedMut {
      inner,
      _contravariant: PhantomData,
    }
  }
}

impl<'t, T> From<Loaned<'t, T>> for LoanedMut<'t, T> {
  #[inline(always)]
  fn from(value: Loaned<'t, T>) -> Self {
    unsafe { LoanedMut::from_inner(value.into_inner()) }
  }
}

impl<'t, T> Drop for LoanedMut<'t, T> {
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

impl<'t, T> Debug for LoanedMut<'t, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
  /// let ab: LoanedMut<(u32, u32)> = LoanedMut::aggregate(Default::default(), |ab, agg| {
  ///   agg.place(a, &mut ab.0);
  ///   agg.place(b, &mut ab.1);
  /// });
  /// ```
  pub fn aggregate(value: T, f: impl for<'i> FnOnce(&'i mut T, &'i AggregatorMut<'t, 'i>)) -> Self {
    unsafe {
      let mut inner = MaybeUninit::new(value);
      f(inner.assume_init_mut(), &AggregatorMut(PhantomData));
      LoanedMut::from_inner(inner)
    }
  }
}

/// See `LoanedMut::aggregate`.
pub struct AggregatorMut<'t, 'i>(PhantomData<(&'t mut &'t (), &'i mut &'i ())>);

impl<'t, 'i> AggregatorMut<'t, 'i> {
  /// See `LoanedMut::aggregate`.
  pub fn place<T>(&'i self, loaned: LoanedMut<'t, T>, place: &'i mut impl Place<'i, T>) {
    place.place(unsafe { LoanedMut::from_inner(loaned.into_inner()) })
  }
}
