use std::ops::{Deref, DerefMut};

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

pub trait LoaneeMut<'t>: Loanee<'t> + DerefMut {}
impl<'t, T: Loanee<'t> + DerefMut> LoaneeMut<'t> for T {}
