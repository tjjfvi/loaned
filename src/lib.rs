#![doc = include_str!("../README.md")]

mod loanable;
mod loaned;
mod loaned_mut;
mod place;

pub use loanable::*;
pub use loaned::*;
pub use loaned_mut::*;
pub use place::*;

#[macro_export]
macro_rules! take {
  ($loaned:expr) => {{
    let mut value = None;
    $loaned.place(&mut value);
    value.unwrap()
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
}
