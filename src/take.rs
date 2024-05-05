use crate::*;

/// Takes the value from a [`Loaned`] or [`LoanedMut`], statically ensuring that
/// `'t` is expired.
///
/// # Example
/// ```
/// use loaned::{take, LoanedMut};
/// let (borrow, loaned) = LoanedMut::loan(Box::new(123));
/// *borrow = 456;
/// assert_eq!(take!(loaned), Box::new(456));
/// ```
#[macro_export]
macro_rules! take {
  ($loaned:expr) => {{
    let mut loaned = ();
    unsafe { $crate::__take($loaned, &mut loaned) }
  }};
}

/// Drops the value from a [`Loaned`] or [`LoanedMut`], statically ensuring that
/// `'t` is expired.
///
/// # Example
/// ```
/// use loaned::{drop, LoanedMut};
/// let (borrow, loaned) = LoanedMut::loan(Box::new(123));
/// *borrow *= 2;
/// assert_eq!(*borrow, 246);
/// drop!(loaned); // drops the box
/// ```
#[macro_export]
macro_rules! drop {
  ($loaned:expr) => {{
    let loaned_value;
    let mut loaned = ();
    loaned_value = unsafe { $crate::__take($loaned, &mut loaned) };
    let _ = loaned_value;
  }};
}

#[doc(hidden)]
pub unsafe fn __take<'t, T: 't, L: Placeable<'t, T>>(loaned: L, _: &'t mut ()) -> T {
  let mut place = MaybeUninit::uninit();
  loaned.place(unsafe { &mut *(&mut place as *mut _) });
  place.assume_init()
}

mod test_drop_cyclic {
  /**
  ```rust
  use loaned::*;

  struct Foo<'a>(&'a ());

  type CyclicFoo<'a> = &'a mut Foo<'a>;
  let (_, loaned): (CyclicFoo, _) = LoanedMut::loan(Box::new(Foo(&())));
  drop!(loaned);
  ```
  */
  mod no_drop {}

  /**
  ```rust
  #![feature(dropck_eyepatch)]
  use loaned::*;

  struct Foo<'a>(&'a ());
  unsafe impl<#[may_dangle] 'a> Drop for Foo<'a> { fn drop(&mut self) {} }

  type CyclicFoo<'a> = &'a mut Foo<'a>;
  let (_, loaned): (CyclicFoo, _) = LoanedMut::loan(Box::new(Foo(&())));
  drop!(loaned);
  ```
  */
  mod drop_eyepatch {}

  /**
  ```rust,compile_fail E0597
  use loaned::*;

  struct Foo<'a>(&'a ());
  impl<'a> Drop for Foo<'a> { fn drop(&mut self) {} }

  type CyclicFoo<'a> = &'a mut Foo<'a>;
  let (_, loaned): (CyclicFoo, _) = LoanedMut::loan(Box::new(Foo(&())));
  drop!(loaned);
  ```
  */
  mod bad_drop {}
}
