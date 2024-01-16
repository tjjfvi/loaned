use crate::*;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::ops::DerefMut;

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
