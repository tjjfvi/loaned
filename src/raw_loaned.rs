use crate::*;

#[repr(C)]
pub(crate) union RawLoaned<T> {
  value: ManuallyDrop<T>,
}

impl<T> RawLoaned<T> {
  pub fn new(value: T) -> Self {
    RawLoaned {
      value: ManuallyDrop::new(value),
    }
  }
  pub unsafe fn as_mut(&mut self) -> &mut T {
    &mut self.value
  }
  pub unsafe fn as_ref(&self) -> &T {
    &self.value
  }
}

impl<T> From<RawLoaned<T>> for MaybeUninit<T> {
  fn from(value: RawLoaned<T>) -> Self {
    unsafe { mem::transmute_copy(&value) }
  }
}
