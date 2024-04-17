use crate::*;
use std::mem::MaybeUninit;

/// The trait underlying `Loaned::place` and `LoanedMut::place`.
pub trait Place<'t, T> {
  #[allow(missing_docs)]
  fn place(&'t mut self, value: LoanedMut<'t, T>);
}

impl<'t, T> Place<'t, T> for MaybeUninit<T> {
  #[inline]
  fn place(&'t mut self, loaned: LoanedMut<'t, T>) {
    *self = loaned.into_inner();
  }
}

impl<'t, T> Place<'t, T> for T {
  #[inline]
  fn place(&'t mut self, loaned: LoanedMut<'t, T>) {
    unsafe {
      let ptr = self as *mut T;
      ptr.read();
      (ptr as *mut MaybeUninit<T>).write(loaned.into_inner());
    }
  }
}

impl<'t, T> Place<'t, T> for Option<T> {
  #[inline]
  fn place(&'t mut self, loaned: LoanedMut<'t, T>) {
    unsafe {
      let ptr = self as *mut Option<T>;
      ptr.read();
      (ptr as *mut MaybeUninit<Option<T>>).write(_maybe_uninit_some(loaned.into_inner()));
    }
  }
}

#[inline(always)]
unsafe fn _maybe_uninit_some<T>(x: MaybeUninit<T>) -> MaybeUninit<Option<T>> {
  // This is somewhat suspicious but seems to make miri happy.
  //
  // We know that `x` is, in some senses, a valid `T` (i.e. it's initialized,
  // and complies with all the layout requirements of `T`), but we can't use it
  // as a `T` -- in particular, if `T` is a `Box<U>`, moving the box invalidates
  // the mutable references we loaned out.
  std::mem::transmute::<_, fn(MaybeUninit<T>) -> MaybeUninit<Option<T>>>(Some::<T> as fn(_) -> _)(x)
}
