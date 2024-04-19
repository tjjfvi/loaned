use crate::*;

/// Abstracts [`Loaned::place`] and [`LoanedMut::place`] for [`take!`] and [`drop!`].
pub trait Placeable<'t, T>: Sized {
  #[allow(missing_docs)]
  fn place(self, place: &'t mut impl Place<'t, T>);
}

impl<'t, T> Placeable<'t, T> for Loaned<'t, T> {
  fn place(self, place: &'t mut impl Place<'t, T>) {
    self.place(place)
  }
}

impl<'t, T> Placeable<'t, T> for LoanedMut<'t, T> {
  fn place(self, place: &'t mut impl Place<'t, T>) {
    self.place(place)
  }
}

/// Types that can be written into with [`Loaned::place`] and [`LoanedMut::place`].
pub trait Place<'t, T> {
  #[allow(missing_docs)]
  fn place(loaned: LoanedMut<'t, T>, place: &'t mut Self);
}

impl<'t, T> Place<'t, T> for MaybeUninit<T> {
  #[inline]
  fn place(loaned: LoanedMut<'t, T>, place: &'t mut Self) {
    *place = loaned.into_raw().into();
  }
}

impl<'t, T> Place<'t, T> for T {
  #[inline]
  fn place(loaned: LoanedMut<'t, T>, place: &'t mut Self) {
    unsafe {
      let ptr = place as *mut T;
      ptr::drop_in_place(ptr);
      ptr.cast::<RawLoaned<T>>().write(loaned.into_raw());
    }
  }
}

impl<'t, T> Place<'t, T> for Option<T> {
  #[inline]
  fn place(loaned: LoanedMut<'t, T>, place: &'t mut Self) {
    unsafe {
      let ptr = place as *mut Option<T>;
      ptr::drop_in_place(ptr);
      ptr
        .cast::<MaybeUninit<Option<T>>>()
        .write(_maybe_uninit_some(loaned.into_raw().into()));
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
  mem::transmute::<_, fn(MaybeUninit<T>) -> MaybeUninit<Option<T>>>(Some::<T> as fn(_) -> _)(x)
}
