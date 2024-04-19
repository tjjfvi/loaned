use crate::*;

macro_rules! main_impls {
  ($Loaned:ident) => {
    #[cfg(feature = "alloc")]
    impl<'t, T> From<Box<$Loaned<'t, T>>> for $Loaned<'t, Box<T>> {
      fn from(value: Box<$Loaned<'t, T>>) -> Self {
        unsafe { $Loaned::new(Box::from_raw(Box::into_raw(value) as *mut _)) }
      }
    }

    #[cfg(feature = "alloc")]
    impl<'t, T> From<Vec<$Loaned<'t, T>>> for $Loaned<'t, Vec<T>> {
      fn from(value: Vec<$Loaned<'t, T>>) -> Self {
        unsafe {
          let mut value = ManuallyDrop::new(value);
          $Loaned::new(Vec::from_raw_parts(
            value.as_mut_ptr() as *mut _,
            value.len(),
            value.capacity(),
          ))
        }
      }
    }

    impl<'t, T, const N: usize> From<[$Loaned<'t, T>; N]> for $Loaned<'t, [T; N]> {
      fn from(value: [$Loaned<'t, T>; N]) -> Self {
        unsafe { mem::transmute_copy(&ManuallyDrop::new(value)) }
      }
    }

    impl<'t, T> From<$Loaned<'t, MaybeUninit<T>>> for MaybeUninit<$Loaned<'t, T>> {
      fn from(value: $Loaned<'t, MaybeUninit<T>>) -> Self {
        unsafe { mem::transmute_copy(&ManuallyDrop::new(value)) }
      }
    }

    impl<'t, T> From<MaybeUninit<$Loaned<'t, T>>> for $Loaned<'t, MaybeUninit<T>> {
      fn from(value: MaybeUninit<$Loaned<'t, T>>) -> Self {
        unsafe { mem::transmute_copy(&ManuallyDrop::new(value)) }
      }
    }
  };
}

main_impls!(Loaned);
main_impls!(LoanedMut);

macro_rules! tuple_impls {
  ($Loaned:ident [$($x:tt)*] $i:tt $T:ident $($y:tt)*) => {
    tuple_impls!($Loaned $($x)* $i $T);
    tuple_impls!($Loaned [$($x)* $i $T] $($y)*);
  };
  ($Loaned:ident [$($x:tt)*]) => {};
  ($Loaned:ident $($i:tt $T:ident)+) => {
    impl<'t, $($T),*> From<($($Loaned<'t, $T>,)*)> for $Loaned<'t, ($($T,)*)> {
      fn from(value: ($($Loaned<'t, $T>,)*)) -> Self {
        unsafe {
          MaybeUninit::from($Loaned::merge(MaybeUninit::<($($T,)*)>::uninit(), |t, m| {
            $(m.place(value.$i, &mut *ptr::addr_of_mut!((*t.as_mut_ptr()).$i).cast::<MaybeUninit<$T>>());)*
          })).assume_init()
        }
      }
    }
  };
}

tuple_impls!(Loaned [] 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H 8 I 9 J);
tuple_impls!(LoanedMut [] 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H 8 I 9 J);
