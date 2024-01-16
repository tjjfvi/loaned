use std::{
  any::type_name,
  marker::PhantomData,
  mem::{ManuallyDrop, MaybeUninit},
  ops::{Deref, DerefMut},
};

#[repr(transparent)]
pub struct MayLeak<T>(pub ManuallyDrop<T>);

impl<T> MayLeak<T> {
  pub fn new(value: T) -> Self {
    MayLeak(ManuallyDrop::new(value))
  }
  pub fn into_inner(slot: Self) -> T {
    ManuallyDrop::into_inner(slot.0)
  }
}

impl<'t, T: Deref> Deref for MayLeak<T> {
  type Target = T::Target;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<'t, T: DerefMut> DerefMut for MayLeak<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

unsafe impl<'t, T: Loanee<'t>> Loanee<'t> for MayLeak<T> {
  const NEEDS_DROP: bool = false;
}

#[repr(transparent)]
pub struct Loaned<'t, T: Loanee<'t>> {
  /// Invariant: the pointee of `inner` is borrowed for `'t`, so it must
  /// not be dropped for the duration of `'t`.
  inner: MaybeUninit<T>,
  // establish contravariance over `'t``
  __: PhantomData<fn(&'t ())>,
}

pub unsafe trait Loanee<'t>: Deref {
  const NEEDS_DROP: bool;
}

unsafe impl<'t, T> Loanee<'t> for Box<T> {
  const NEEDS_DROP: bool = true;
}
unsafe impl<'t, T> Loanee<'t> for std::rc::Rc<T> {
  const NEEDS_DROP: bool = true;
}
unsafe impl<'t, T> Loanee<'t> for std::sync::Arc<T> {
  const NEEDS_DROP: bool = true;
}
unsafe impl<'t, 'a: 't, T> Loanee<'t> for &'a T {
  const NEEDS_DROP: bool = false;
}
unsafe impl<'t, 'a: 't, T> Loanee<'t> for &'a mut T {
  const NEEDS_DROP: bool = false;
}

pub unsafe trait Place<'t, T> {
  unsafe fn place(&'t mut self, value: MaybeUninit<T>);
}

unsafe impl<'t, T> Place<'t, T> for T {
  unsafe fn place(&'t mut self, value: MaybeUninit<T>) {
    let ptr = self as *mut T;
    ptr.read();
    (ptr as *mut MaybeUninit<T>).write(value);
  }
}

unsafe impl<'t, T> Place<'t, MayLeak<T>> for T {
  unsafe fn place(&'t mut self, value: MaybeUninit<MayLeak<T>>) {
    self.place(std::mem::transmute_copy::<_, MaybeUninit<T>>(&value))
  }
}

unsafe impl<'t, T> Place<'t, T> for Option<T> {
  unsafe fn place(&'t mut self, value: MaybeUninit<T>) {
    let ptr = self as *mut Option<T>;
    ptr.read();
    (ptr as *mut MaybeUninit<Option<T>>).write(quiet_some(value));
  }
}

unsafe impl<'t, T> Place<'t, MayLeak<T>> for Option<T> {
  unsafe fn place(&'t mut self, value: MaybeUninit<MayLeak<T>>) {
    self.place(std::mem::transmute_copy::<_, MaybeUninit<T>>(&value))
  }
}

fn quiet_some<T>(x: MaybeUninit<T>) -> MaybeUninit<Option<T>> {
  unsafe {
    std::mem::transmute::<_, fn(MaybeUninit<T>) -> MaybeUninit<Option<T>>>(Some::<T> as fn(_) -> _)(
      x,
    )
  }
}

impl<'t, T: Loanee<'t>> Loaned<'t, T> {
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

impl<'t, T: Loanee<'t>> Deref for Loaned<'t, T> {
  type Target = T::Target;
  fn deref(&self) -> &T::Target {
    unsafe { &*(&**self.inner.assume_init_ref() as *const _) }
  }
}

impl<'t, T: Loanee<'t>> Drop for Loaned<'t, T> {
  fn drop(&mut self) {
    if T::NEEDS_DROP && !std::thread::panicking() {
      panic!(
        "memory leak: cannot drop `Loaned<{T}>`
    if leaking is desired, use `Loaned<MayLeak<{T}>>` or `mem::forget`
    otherwise, use `loaned.place(&mut None)` to drop the inner value",
        T = type_name::<T>()
      )
    }
  }
}

pub struct LoanedMut<'t, T: LoaneeMut<'t>> {
  /// Invariant: the pointee of `inner` is mutably borrowed for `'t`, so it must
  /// not be dropped for the duration of `'t`.
  inner: MaybeUninit<T>,
  // establish contravariance over `'t``
  __: PhantomData<fn(&'t ())>,
}

pub trait LoaneeMut<'t>: Loanee<'t> + DerefMut {}
impl<'t, T: Loanee<'t> + DerefMut> LoaneeMut<'t> for T {}

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

impl<'t, T: LoaneeMut<'t>> From<Loaned<'t, T>> for LoanedMut<'t, T> {
  fn from(value: Loaned<'t, T>) -> Self {
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
        T = type_name::<T>()
      )
    }
  }
}

#[test]
fn test() {
  use std::sync::atomic::{AtomicU32, Ordering};
  let (r, b) = LoanedMut::new(Box::new(0));
  *r = 1;
  let mut x = Box::new(0);
  b.place(&mut x);
  *r = 2;
  assert_eq!(*x, 2);

  let (r, b) = Loaned::new(MayLeak::new(Box::new(AtomicU32::new(0))));
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
