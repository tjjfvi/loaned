//! A typed arena allocator implementation with no unsafe code or interior
//! mutability.
//!
//! API inspired by the [`typed_arena`] crate.
//!
//! [`typed_arena`]: https://docs.rs/typed-arena/latest/typed_arena/

use std::{cell::Cell, mem};

use loaned::{drop, LoanedMut};

pub struct Arena<'t, T> {
  cursor: &'t mut [Option<T>],
  chunks: Vec<LoanedMut<'t, Box<[Option<T>]>>>,
  capacity: usize,
}

impl<'t, T> Arena<'t, T> {
  pub fn with_capacity(capacity: usize) -> Self {
    let capacity = capacity.max(1);
    let (cursor, chunk) = Self::new_chunk(capacity);
    Arena {
      cursor,
      chunks: vec![chunk],
      capacity,
    }
  }

  fn new_chunk(capacity: usize) -> (&'t mut [Option<T>], LoanedMut<'t, Box<[Option<T>]>>) {
    let mut chunk = Vec::with_capacity(capacity);
    chunk.resize_with(capacity, || None);
    let chunk = chunk.into_boxed_slice();
    let (cursor, chunk) = LoanedMut::loan(chunk);
    (cursor, chunk)
  }

  pub fn alloc(&mut self, value: T) -> &'t mut T {
    if self.cursor.is_empty() {
      self.capacity *= 2;
      let (cursor, chunk) = Self::new_chunk(self.capacity);
      self.cursor = cursor;
      self.chunks.push(chunk);
    }
    let cursor = mem::replace(&mut self.cursor, &mut []);
    let (slot, cursor) = cursor.split_first_mut().unwrap();
    self.cursor = cursor;
    *slot = Some(value);
    let Some(slot) = slot else { unreachable!() };
    slot
  }

  pub fn into_inner(self) -> LoanedMut<'t, Vec<Box<[Option<T>]>>> {
    self.chunks.into()
  }
}

#[cfg_attr(test, test)]
fn main() {
  let mut arena = Arena::with_capacity(1);

  struct CycleParticipant<'a> {
    name: &'static str,
    next: Cell<Option<&'a CycleParticipant<'a>>>,
  }

  impl<'a> CycleParticipant<'a> {
    fn next(&self) -> &'a CycleParticipant<'a> {
      self.next.get().unwrap()
    }
  }

  let a = arena.alloc(CycleParticipant {
    name: "a",
    next: Cell::new(None),
  });

  let b = arena.alloc(CycleParticipant {
    name: "b",
    next: Cell::new(None),
  });

  a.next.set(Some(b));
  b.next.set(Some(a));

  print_assert_eq!(a.name, "a");
  print_assert_eq!(a.next().name, "b");
  print_assert_eq!(a.next().next().name, "a");

  drop!(arena.into_inner());
}

#[macro_export]
macro_rules! print_assert_eq {
  ($x:expr, $y:expr) => {
    println!("{} = {:?}", stringify!($x), $x);
    assert_eq!($x, $y);
  };
}
