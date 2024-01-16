use std::ops::{Deref, DerefMut};

pub unsafe trait Loanable<'t>: Deref {
  const NEEDS_DROP: bool;
}

unsafe impl<'t, T> Loanable<'t> for Box<T> {
  const NEEDS_DROP: bool = true;
}

unsafe impl<'t, T> Loanable<'t> for std::rc::Rc<T> {
  const NEEDS_DROP: bool = true;
}

unsafe impl<'t, T> Loanable<'t> for std::sync::Arc<T> {
  const NEEDS_DROP: bool = true;
}

unsafe impl<'t, 'a: 't, T> Loanable<'t> for &'a T {
  const NEEDS_DROP: bool = false;
}

unsafe impl<'t, 'a: 't, T> Loanable<'t> for &'a mut T {
  const NEEDS_DROP: bool = false;
}

pub trait LoaneeMut<'t>: Loanable<'t> + DerefMut {}
impl<'t, T: Loanable<'t> + DerefMut> LoaneeMut<'t> for T {}
