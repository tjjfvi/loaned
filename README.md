# `loaned`

`loaned` provides `Loaned<'t, T>` and `LoanedMut<'t, T>` types which allow
owning values that have live immutable/mutable borrows, allowing a limited (but
very expressive) subset of self-referential structures to be expressed.

## Safety

`loaned` uses unsafe code, the soundness of which has not been rigorously
proven, although basic tests have been run successfully through Miri.
