use crate::*;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

pub struct LoanedMut<'t, T: LoaneeMut<'t>> {
  /// Invariant: the pointee of `inner` is mutably borrowed for `'t`, so it must
  /// not be dropped for the duration of `'t`.
  pub(crate) inner: MaybeUninit<T>,
  // establish contravariance over `'t``
  pub(crate) __: PhantomData<fn(&'t ())>,
}

impl<'t, T: LoaneeMut<'t>> LoanedMut<'t, T> {
  pub fn new(value: T) -> (&'t mut T::Target, Self) {
    let mut inner = MaybeUninit::new(value);
    let borrow = unsafe { &mut *(&mut **inner.assume_init_mut() as *mut _) };
    (
      borrow,
      LoanedMut {
        inner,
        __: PhantomData,
      },
    )
  }
  pub fn place(self, place: &'t mut impl Place<'t, T>) {
    unsafe { place.place(self.into_inner()) }
  }
  pub fn into_inner(mut self) -> MaybeUninit<T> {
    let inner = std::mem::replace(&mut self.inner, MaybeUninit::uninit());
    std::mem::forget(self);
    inner
  }
}

impl<'t, T: LoaneeMut<'t>> From<loaned::Loaned<'t, T>> for LoanedMut<'t, T> {
  fn from(value: loaned::Loaned<'t, T>) -> Self {
    LoanedMut {
      inner: value.into_inner(),
      __: PhantomData,
    }
  }
}

impl<'t, T: LoaneeMut<'t>> Drop for LoanedMut<'t, T> {
  fn drop(&mut self) {
    if T::NEEDS_DROP && !std::thread::panicking() {
      panic!(
        "memory leak: cannot drop `LoanedMut<{T}>`
    if leaking is desired, use `LoanedMut<MayLeak<{T}>>` or `mem::forget`
    otherwise, use `loaned.place(&mut None)` to drop the inner value",
        T = std::any::type_name::<T>()
      )
    }
  }
}
