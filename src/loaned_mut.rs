use crate::*;
use std::{
  fmt::Debug,
  marker::PhantomData,
  mem::{ManuallyDrop, MaybeUninit},
  ops::DerefMut,
};

/// `LoanedMut<'t, T>` connotes ownership of a smart pointer `T`, with the
/// caveat that its target is mutably loaned for `'t` (i.e. something else may
/// hold an `&'t mut` reference to the target of this pointer).
///
/// Thus, for the duration of `'t`, one cannot access the target of this
/// pointer.
///
/// The main thing one *can* do is store this smart pointer somewhere, as long
/// as its ensured that it won't be used for the duration of `'t`.
///
/// This can be done with `LoanedMut::place`, which can store the smart pointer
/// into an `&'t mut T` (among other things).
///
/// # Dropping
///
/// The smart pointer held by a `LoanedMut` can only be dropped once `'t`
/// expires. Since there is no way in the type system to enforce this, nor any
/// way to check this at runtime, dropping a `LoanedMut` panics.
///
/// If leaking is intentional, use a `ManuallyDrop<LoanedMut<'t, T>>`.
///
/// Otherwise, use `loaned.place(&mut None)` to drop the inner value.
#[must_use]
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

  /// Inserts the contained smart pointer into a given place. See the `Place`
  /// trait for more.
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
    otherwise, use `loaned.place(&mut None)` to drop the inner value",
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
  pub fn aggregate(value: T, f: impl for<'i> FnOnce(&'i mut T, &'i AggregatorMut<'t, 'i>)) -> Self {
    unsafe {
      let mut inner = MaybeUninit::new(value);
      f(inner.assume_init_mut(), &AggregatorMut(PhantomData));
      LoanedMut::from_inner(inner)
    }
  }
}

pub struct AggregatorMut<'t, 'i>(PhantomData<(&'t mut &'t (), &'i mut &'i ())>);

impl<'t, 'i> AggregatorMut<'t, 'i> {
  pub fn place<T>(&'i self, loaned: LoanedMut<'t, T>, place: &'i mut impl Place<'i, T>) {
    place.place(unsafe { LoanedMut::from_inner(loaned.into_inner()) })
  }
}
