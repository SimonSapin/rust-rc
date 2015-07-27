# rust-rc

A copy of the [http://doc.rust-lang.org/std/rc/](`std::rc`) module
that runs on stable Rust with `Weak` references.

As of this writing, `std::rc::Weak` is marked `#[unstable]`
and therefore can not be used on stable Rust yet.

To make this work, some features had to be removed:

* Unsized / dynamically-sized types `T` are not supported in `Rc<T>` or `Weak<T>`
* `#[unsafe_no_drop_flag]` is not used,
  so (in curent Rust) `Rc<T>` and `Weak<T>` have a drop flag
  and are two words big (16 bytes 64-bit platforms) instead of one.
* `NonZero` is not used,
  so `Option<Rc<T>>` and `Option<Weak<T>>` are one word bigger than `Rc<T>` or `Weak<T>`
  (for a total of 24 bytes instead of 8 on 64-bit platforms).
* `std::intrinsics::assume` is not used,
  so the optimizer may not be able to remove as many redundant checks.
