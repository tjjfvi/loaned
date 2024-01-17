use std::ops::Deref;

/// The trait for types that can be used with `Loaned::loan` and `LoanedMut::loan`.
///
/// # Safety
///
/// This type must ensure that the reference returned by `Deref` (and
/// `DerefMut`, if implemented) is valid for `'t`, as long as `self` is not used
/// for the remainder of `'t` (though it may be moved).
///
/// In particular, this can't be implemented for types like `Cow`, as it may
/// return a reference to data within `self` (which would be invalidated when
/// `self` is moved).
///
/// This is closely related to whether the type can unconditionally implement
/// `Unpin` (i.e. even when `Self::Target: !Unpin`).
pub unsafe trait Loanable<'t>: Deref {
  /// Whether or not this type should be dropped. When this is `true`, dropping
  /// a `Loaned<Self>` will panic instead of leaking. This behavior can be opted
  /// out of with `MayLeak`.
  ///
  /// Note that Rust never guarantees that `Drop` will be called, so even when
  /// this is `true`, the value may still be leaked.
  const NEEDS_DROP: bool;
}

unsafe impl<'t, T: ?Sized> Loanable<'t> for Box<T> {
  const NEEDS_DROP: bool = true;
}

unsafe impl<'t, T> Loanable<'t> for Vec<T> {
  const NEEDS_DROP: bool = true;
}

unsafe impl<'t> Loanable<'t> for String {
  const NEEDS_DROP: bool = true;
}

unsafe impl<'t, T: ?Sized> Loanable<'t> for std::rc::Rc<T> {
  const NEEDS_DROP: bool = true;
}

unsafe impl<'t, T: ?Sized> Loanable<'t> for std::sync::Arc<T> {
  const NEEDS_DROP: bool = true;
}

// The usefulness of this implementation is dubious at best, but it's here for completeness.
unsafe impl<'t, 'a: 't, T: ?Sized> Loanable<'t> for &'a T {
  const NEEDS_DROP: bool = false;
}

unsafe impl<'t, 'a: 't, T: ?Sized> Loanable<'t> for &'a mut T {
  const NEEDS_DROP: bool = false;
}
