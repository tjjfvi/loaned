use crate::*;
use std::mem::MaybeUninit;

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
    (ptr as *mut MaybeUninit<Option<T>>).write(_some(value));
  }
}

unsafe impl<'t, T> Place<'t, MayLeak<T>> for Option<T> {
  unsafe fn place(&'t mut self, value: MaybeUninit<MayLeak<T>>) {
    self.place(std::mem::transmute_copy::<_, MaybeUninit<T>>(&value))
  }
}

fn _some<T>(x: MaybeUninit<T>) -> MaybeUninit<Option<T>> {
  unsafe {
    std::mem::transmute::<_, fn(MaybeUninit<T>) -> MaybeUninit<Option<T>>>(Some::<T> as fn(_) -> _)(
      x,
    )
  }
}
