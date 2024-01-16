use crate::*;
use std::{marker::PhantomData, mem::MaybeUninit, ops::Deref};

#[repr(transparent)]
pub struct Loaned<'t, T: Loanable<'t>> {
  /// Invariant: the pointee of `inner` is borrowed for `'t`, so it must
  /// not be dropped for the duration of `'t`.
  pub(crate) inner: MaybeUninit<T>,
  // establish contravariance over `'t``
  pub(crate) __: PhantomData<fn(&'t ())>,
}

impl<'t, T: Loanable<'t>> Loaned<'t, T> {
  pub fn new(value: T) -> (&'t T::Target, Self) {
    let inner = MaybeUninit::new(value);
    let loaned = Loaned {
      inner,
      __: PhantomData,
    };
    (loaned.borrow(), loaned)
  }
  pub fn place(self, place: &'t mut impl Place<'t, T>) {
    unsafe { place.place(self.into_inner()) }
  }
  pub fn into_inner(mut self) -> MaybeUninit<T> {
    let inner = std::mem::replace(&mut self.inner, MaybeUninit::uninit());
    std::mem::forget(self);
    inner
  }
  pub fn borrow(&self) -> &'t T::Target {
    unsafe { &*(&**self.inner.assume_init_ref() as *const _) }
  }
}

impl<'t, T: Loanable<'t>> Deref for Loaned<'t, T> {
  type Target = T::Target;
  fn deref(&self) -> &T::Target {
    unsafe { &*(&**self.inner.assume_init_ref() as *const _) }
  }
}

impl<'t, T: Loanable<'t>> Drop for Loaned<'t, T> {
  fn drop(&mut self) {
    if T::NEEDS_DROP && !std::thread::panicking() {
      panic!(
        "memory leak: cannot drop `Loaned<{T}>`
    if leaking is desired, use `Loaned<MayLeak<{T}>>` or `mem::forget`
    otherwise, use `loaned.place(&mut None)` to drop the inner value",
        T = std::any::type_name::<T>()
      )
    }
  }
}
