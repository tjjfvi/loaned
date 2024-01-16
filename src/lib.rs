mod loaned;
mod loaned_mut;
mod loanee;
mod may_leak;
mod place;

pub use loaned::*;
pub use loaned_mut::*;
pub use loanee::*;
pub use may_leak::*;
pub use place::*;

#[test]
fn test() {
  use std::sync::atomic::{AtomicU32, Ordering};
  let (r, b) = LoanedMut::new(Box::new(0));
  *r = 1;
  let mut x = Box::new(0);
  b.place(&mut x);
  *r = 2;
  assert_eq!(*x, 2);

  let (r, b) = loaned::Loaned::new(MayLeak::new(Box::new(AtomicU32::new(0))));
  r.fetch_add(1, Ordering::Relaxed);
  b.borrow().fetch_add(2, Ordering::Relaxed);
  assert_eq!(b.load(Ordering::Relaxed), 3);
  let mut x = Box::new(AtomicU32::new(0));
  b.place(&mut x);
  r.fetch_add(4, Ordering::Relaxed);
  *x.as_mut().get_mut() += 5;
  assert_eq!(*x.as_mut().get_mut(), 12);

  let (r, b) = LoanedMut::new(Box::new(123));
  *r = 1;
  let mut x = None;
  b.place(&mut x);
  *r = 2;
  assert_eq!(x, Some(Box::new(2)));
}
