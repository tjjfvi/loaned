#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod loanable;
mod loaned;
mod loaned_mut;
mod place;

pub use loanable::*;
pub use loaned::*;
pub use loaned_mut::*;
pub use place::*;

/// Takes the value from a `Loaned` or `LoanedMut`, statically ensuring that
/// `'t` is expired.
#[macro_export]
macro_rules! take {
  ($loaned:expr) => {{
    let mut value = None;
    $loaned.place(&mut value);
    value.unwrap()
  }};
}

/// Drops the value from a `Loaned` or `LoanedMut`, statically ensuring that
/// `'t` is expired.
#[macro_export]
macro_rules! drop {
  ($loaned:expr) => {{
    $loaned.place(&mut None);
  }};
}

#[cfg(test)]
mod test {
  use super::*;
  use std::sync::atomic::{AtomicU32, Ordering};

  #[test]
  fn loaned_mut() {
    let (r, b) = LoanedMut::loan(Box::new(0));
    *r = 1;
    let mut x = Box::new(0);
    b.place(&mut x);
    *r = 2;
    assert_eq!(*x, 2);
  }

  #[test]
  fn loaned_atomic() {
    let (r, b) = Loaned::loan(Box::new(AtomicU32::new(0)));
    r.fetch_add(1, Ordering::Relaxed);
    b.borrow().fetch_add(2, Ordering::Relaxed);
    assert_eq!(b.load(Ordering::Relaxed), 3);
    let mut x = Box::new(AtomicU32::new(0));
    b.place(&mut x);
    r.fetch_add(4, Ordering::Relaxed);
    *x.as_mut().get_mut() += 5;
    assert_eq!(*x.as_mut().get_mut(), 12);
  }

  #[test]
  fn place_option() {
    let (r, b) = LoanedMut::loan(Box::new(123));
    *r = 1;
    let mut x = None;
    b.place(&mut x);
    *r = 2;
    assert_eq!(x, Some(Box::new(2)));
  }

  #[test]
  fn take() {
    let (r, b) = LoanedMut::loan(Box::new(123));
    *r = 1;
    *r = 2;
    assert_eq!(take!(b), Box::new(2));
  }

  #[test]
  fn take_merge() {
    let (r1, b1) = LoanedMut::loan(Box::new(123));
    let (r2, b2) = LoanedMut::loan(Box::new(123));
    *r1 = 1;
    let a = LoanedMut::<(Box<i32>, Box<i32>)>::merge(Default::default(), |x, m| {
      m.place(b1, &mut x.0);
      m.place(b2, &mut x.1);
    });
    *r2 = 2;
    assert_eq!(take!(a), (Box::new(1), Box::new(2)));
  }

  #[test]
  fn into_box() {
    let (r, b) = LoanedMut::loan(Box::new(123));
    *r = 1;
    let x = LoanedMut::<Box<Box<_>>>::from(Box::new(b));
    *r = 2;
    assert_eq!(take!(x), Box::new(Box::new(2)));
  }

  #[test]
  fn into_vec() {
    let (r1, b1) = LoanedMut::loan(Box::new(123));
    let (r2, b2) = LoanedMut::loan(Box::new(123));
    let (r3, b3) = LoanedMut::loan(Box::new(123));
    let mut x = Vec::new();
    x.push(b1);
    *r1 = 1;
    *r2 = 2;
    x.push(b2);
    x.push(b3);
    let x: LoanedMut<Vec<Box<_>>> = x.into();
    *r3 = 3;
    assert_eq!(take!(x), vec![Box::new(1), Box::new(2), Box::new(3)]);
  }
}
