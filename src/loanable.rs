use crate::*;

/// The trait for types that can be used with [`Loaned::loan`] and
/// [`LoanedMut::loan`].
///
/// # Safety
///
/// This type must ensure that the reference returned by [`Deref`] (and
/// [`DerefMut`], if implemented) is valid for `'t`, as long as `self` is not
/// used for the remainder of `'t` (though it may be moved).
///
/// In particular, this can't be implemented for types like `Cow`, as it may
/// return a reference to data within `self` (which would be invalidated when
/// `self` is moved).
///
/// This is closely related to whether the type can unconditionally implement
/// [`Unpin`] (i.e. even when `Self::Target: !Unpin`).
pub unsafe trait Loanable<'t>: Deref {}

#[cfg(feature = "alloc")]
mod _alloc {
  use crate::*;
  unsafe impl<'t, T: ?Sized> Loanable<'t> for alloc::boxed::Box<T> {}
  unsafe impl<'t, T> Loanable<'t> for alloc::vec::Vec<T> {}
  unsafe impl<'t> Loanable<'t> for alloc::string::String {}
  unsafe impl<'t, T: ?Sized> Loanable<'t> for alloc::rc::Rc<T> {}
  unsafe impl<'t, T: ?Sized> Loanable<'t> for alloc::sync::Arc<T> {}
}

// The usefulness of this implementation is dubious at best, but it's here for completeness.
unsafe impl<'t, 'a: 't, T: ?Sized> Loanable<'t> for &'a T {}
unsafe impl<'t, 'a: 't, T: ?Sized> Loanable<'t> for &'a mut T {}
