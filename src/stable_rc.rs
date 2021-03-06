// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hasher, Hash};
use std::mem::{self, forget};
use std::ops::Deref;
use std::ptr;

struct RcBox<T> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: T,
}

unsafe fn deallocate<T>(ptr: *mut T, count: usize) {
    mem::drop(Vec::from_raw_parts(ptr, 0, count))
}


/// A reference-counted pointer type over an immutable value.
///
/// See the [module level documentation](./index.html) for more details.
pub struct Rc<T> {
    // FIXME #12808: strange names to try to avoid interfering with field
    // accesses of the contained type via Deref
    _ptr: *mut RcBox<T>,
}

impl<T> Rc<T> {
    /// Constructs a new `Rc<T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    /// ```
    pub fn new(value: T) -> Rc<T> {
        let mut rc_box = Box::new(RcBox {
            strong: Cell::new(1),
            weak: Cell::new(1),
            value: value
        });
        let rc = Rc {
            // there is an implicit weak pointer owned by all the strong
            // pointers, which ensures that the weak destructor never frees
            // the allocation while the strong destructor is running, even
            // if the weak pointer is stored inside the strong one.
            _ptr: &mut *rc_box,
        };
        mem::forget(rc_box);
        rc
    }

    /// Unwraps the contained value if the `Rc<T>` is unique.
    ///
    /// If the `Rc<T>` is not unique, an `Err` is returned with the same
    /// `Rc<T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let x = Rc::new(3);
    /// assert_eq!(Rc::try_unwrap(x), Ok(3));
    ///
    /// let x = Rc::new(4);
    /// let _y = x.clone();
    /// assert_eq!(Rc::try_unwrap(x), Err(Rc::new(4)));
    /// ```
    #[inline]
    pub fn try_unwrap(rc: Rc<T>) -> Result<T, Rc<T>> {
        if Rc::is_unique(&rc) {
            unsafe {
                let val = ptr::read(&*rc); // copy the contained object
                // destruct the box and skip our Drop
                // we can ignore the refcounts because we know we're unique
                deallocate(rc._ptr, 1);
                forget(rc);
                Ok(val)
            }
        } else {
            Err(rc)
        }
    }
}

impl<T> Rc<T> {
    /// Downgrades the `Rc<T>` to a `Weak<T>` reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// let weak_five = five.downgrade();
    /// ```
    pub fn downgrade(&self) -> Weak<T> {
        self.inc_weak();
        Weak { _ptr: self._ptr }
    }

    /// Get the number of weak references to this value.
    #[inline]
    pub fn weak_count(this: &Rc<T>) -> usize { this.weak() - 1 }

    /// Get the number of strong references to this value.
    #[inline]
    pub fn strong_count(this: &Rc<T>) -> usize { this.strong() }

    /// Returns true if there are no other `Rc` or `Weak<T>` values that share
    /// the same inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// assert!(Rc::is_unique(&five));
    /// ```
    #[inline]
    pub fn is_unique(rc: &Rc<T>) -> bool {
        Rc::weak_count(rc) == 0 && Rc::strong_count(rc) == 1
    }

    /// Returns a mutable reference to the contained value if the `Rc<T>` is
    /// unique.
    ///
    /// Returns `None` if the `Rc<T>` is not unique.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let mut x = Rc::new(3);
    /// *Rc::get_mut(&mut x).unwrap() = 4;
    /// assert_eq!(*x, 4);
    ///
    /// let _y = x.clone();
    /// assert!(Rc::get_mut(&mut x).is_none());
    /// ```
    #[inline]
    pub fn get_mut(rc: &mut Rc<T>) -> Option<&mut T> {
        if Rc::is_unique(rc) {
            let inner = unsafe { &mut *rc._ptr };
            Some(&mut inner.value)
        } else {
            None
        }
    }
}

/// Get the number of weak references to this value.
#[inline]
pub fn weak_count<T>(this: &Rc<T>) -> usize { Rc::weak_count(this) }

/// Get the number of strong references to this value.
#[inline]
pub fn strong_count<T>(this: &Rc<T>) -> usize { Rc::strong_count(this) }

/// Returns true if there are no other `Rc` or `Weak<T>` values that share the
/// same inner value.
///
/// # Examples
///
/// ```
/// use rc;
/// use rc::Rc;
///
/// let five = Rc::new(5);
///
/// rc::is_unique(&five);
/// ```
#[inline]
pub fn is_unique<T>(rc: &Rc<T>) -> bool { Rc::is_unique(rc) }

/// Unwraps the contained value if the `Rc<T>` is unique.
///
/// If the `Rc<T>` is not unique, an `Err` is returned with the same `Rc<T>`.
///
/// # Examples
///
/// ```
/// use rc::{self, Rc};
///
/// let x = Rc::new(3);
/// assert_eq!(rc::try_unwrap(x), Ok(3));
///
/// let x = Rc::new(4);
/// let _y = x.clone();
/// assert_eq!(rc::try_unwrap(x), Err(Rc::new(4)));
/// ```
#[inline]
pub fn try_unwrap<T>(rc: Rc<T>) -> Result<T, Rc<T>> { Rc::try_unwrap(rc) }

/// Returns a mutable reference to the contained value if the `Rc<T>` is unique.
///
/// Returns `None` if the `Rc<T>` is not unique.
///
/// # Examples
///
/// ```
/// use rc::{self, Rc};
///
/// let mut x = Rc::new(3);
/// *rc::get_mut(&mut x).unwrap() = 4;
/// assert_eq!(*x, 4);
///
/// let _y = x.clone();
/// assert!(rc::get_mut(&mut x).is_none());
/// ```
#[inline]
pub fn get_mut<T>(rc: &mut Rc<T>) -> Option<&mut T> { Rc::get_mut(rc) }

impl<T: Clone> Rc<T> {
    /// Make a mutable reference from the given `Rc<T>`.
    ///
    /// This is also referred to as a copy-on-write operation because the inner
    /// data is cloned if the reference count is greater than one.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let mut five = Rc::new(5);
    ///
    /// let mut_five = five.make_unique();
    /// ```
    #[inline]
    pub fn make_unique(&mut self) -> &mut T {
        if !Rc::is_unique(self) {
            *self = Rc::new((**self).clone())
        }
        // This unsafety is ok because we're guaranteed that the pointer
        // returned is the *only* pointer that will ever be returned to T. Our
        // reference count is guaranteed to be 1 at this point, and we required
        // the `Rc<T>` itself to be `mut`, so we're returning the only possible
        // reference to the inner value.
        let inner = unsafe { &mut *self._ptr };
        &mut inner.value
    }
}

impl<T> Deref for Rc<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.inner().value
    }
}

impl<T> Drop for Rc<T> {
    /// Drops the `Rc<T>`.
    ///
    /// This will decrement the strong reference count. If the strong reference
    /// count becomes zero and the only other references are `Weak<T>` ones,
    /// `drop`s the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// {
    ///     let five = Rc::new(5);
    ///
    ///     // stuff
    ///
    ///     drop(five); // explicit drop
    /// }
    /// {
    ///     let five = Rc::new(5);
    ///
    ///     // stuff
    ///
    /// } // implicit drop
    /// ```
    fn drop(&mut self) {
        unsafe {
            let ptr = self._ptr;
            if !(*(&ptr as *const _ as *const *const ())).is_null() {
                self.dec_strong();
                if self.strong() == 0 {
                    // destroy the contained object
                    mem::drop(ptr::read(&(*ptr).value));

                    // remove the implicit "strong weak" pointer now that we've
                    // destroyed the contents.
                    self.dec_weak();

                    if self.weak() == 0 {
                        deallocate(ptr, 1)
                    }
                }
            }
        }
    }
}

impl<T> Clone for Rc<T> {

    /// Makes a clone of the `Rc<T>`.
    ///
    /// When you clone an `Rc<T>`, it will create another pointer to the data and
    /// increase the strong reference counter.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// five.clone();
    /// ```
    #[inline]
    fn clone(&self) -> Rc<T> {
        self.inc_strong();
        Rc { _ptr: self._ptr }
    }
}

impl<T: Default> Default for Rc<T> {
    /// Creates a new `Rc<T>`, with the `Default` value for `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let x: Rc<i32> = Default::default();
    /// ```
    #[inline]
    fn default() -> Rc<T> {
        Rc::new(Default::default())
    }
}

impl<T: PartialEq> PartialEq for Rc<T> {
    /// Equality for two `Rc<T>`s.
    ///
    /// Two `Rc<T>`s are equal if their inner value are equal.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// five == Rc::new(5);
    /// ```
    #[inline(always)]
    fn eq(&self, other: &Rc<T>) -> bool { **self == **other }

    /// Inequality for two `Rc<T>`s.
    ///
    /// Two `Rc<T>`s are unequal if their inner value are unequal.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// five != Rc::new(5);
    /// ```
    #[inline(always)]
    fn ne(&self, other: &Rc<T>) -> bool { **self != **other }
}

impl<T: Eq> Eq for Rc<T> {}

impl<T: PartialOrd> PartialOrd for Rc<T> {
    /// Partial comparison for two `Rc<T>`s.
    ///
    /// The two are compared by calling `partial_cmp()` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// five.partial_cmp(&Rc::new(5));
    /// ```
    #[inline(always)]
    fn partial_cmp(&self, other: &Rc<T>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }

    /// Less-than comparison for two `Rc<T>`s.
    ///
    /// The two are compared by calling `<` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// five < Rc::new(5);
    /// ```
    #[inline(always)]
    fn lt(&self, other: &Rc<T>) -> bool { **self < **other }

    /// 'Less-than or equal to' comparison for two `Rc<T>`s.
    ///
    /// The two are compared by calling `<=` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// five <= Rc::new(5);
    /// ```
    #[inline(always)]
    fn le(&self, other: &Rc<T>) -> bool { **self <= **other }

    /// Greater-than comparison for two `Rc<T>`s.
    ///
    /// The two are compared by calling `>` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// five > Rc::new(5);
    /// ```
    #[inline(always)]
    fn gt(&self, other: &Rc<T>) -> bool { **self > **other }

    /// 'Greater-than or equal to' comparison for two `Rc<T>`s.
    ///
    /// The two are compared by calling `>=` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// five >= Rc::new(5);
    /// ```
    #[inline(always)]
    fn ge(&self, other: &Rc<T>) -> bool { **self >= **other }
}

impl<T: Ord> Ord for Rc<T> {
    /// Comparison for two `Rc<T>`s.
    ///
    /// The two are compared by calling `cmp()` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// five.partial_cmp(&Rc::new(5));
    /// ```
    #[inline]
    fn cmp(&self, other: &Rc<T>) -> Ordering { (**self).cmp(&**other) }
}

impl<T: Hash> Hash for Rc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: fmt::Display> fmt::Display for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: fmt::Debug> fmt::Debug for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> fmt::Pointer for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self._ptr, f)
    }
}

/// A weak version of `Rc<T>`.
///
/// Weak references do not count when determining if the inner value should be
/// dropped.
///
/// See the [module level documentation](./index.html) for more.
pub struct Weak<T> {
    // FIXME #12808: strange names to try to avoid interfering with
    // field accesses of the contained type via Deref
    _ptr: *mut RcBox<T>,
}

impl<T> Weak<T> {

    /// Upgrades a weak reference to a strong reference.
    ///
    /// Upgrades the `Weak<T>` reference to an `Rc<T>`, if possible.
    ///
    /// Returns `None` if there were no strong references and the data was
    /// destroyed.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let five = Rc::new(5);
    ///
    /// let weak_five = five.downgrade();
    ///
    /// let strong_five: Option<Rc<_>> = weak_five.upgrade();
    /// ```
    pub fn upgrade(&self) -> Option<Rc<T>> {
        if self.strong() == 0 {
            None
        } else {
            self.inc_strong();
            Some(Rc { _ptr: self._ptr })
        }
    }
}

impl<T> Drop for Weak<T> {
    /// Drops the `Weak<T>`.
    ///
    /// This will decrement the weak reference count.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// {
    ///     let five = Rc::new(5);
    ///     let weak_five = five.downgrade();
    ///
    ///     // stuff
    ///
    ///     drop(weak_five); // explicit drop
    /// }
    /// {
    ///     let five = Rc::new(5);
    ///     let weak_five = five.downgrade();
    ///
    ///     // stuff
    ///
    /// } // implicit drop
    /// ```
    fn drop(&mut self) {
        unsafe {
            let ptr = self._ptr;
            if !(*(&ptr as *const _ as *const *const ())).is_null() {
                self.dec_weak();
                // the weak count starts at 1, and will only go to zero if all
                // the strong pointers have disappeared.
                if self.weak() == 0 {
                    deallocate(ptr, 1)
                }
            }
        }
    }
}

impl<T> Clone for Weak<T> {

    /// Makes a clone of the `Weak<T>`.
    ///
    /// This increases the weak reference count.
    ///
    /// # Examples
    ///
    /// ```
    /// use rc::Rc;
    ///
    /// let weak_five = Rc::new(5).downgrade();
    ///
    /// weak_five.clone();
    /// ```
    #[inline]
    fn clone(&self) -> Weak<T> {
        self.inc_weak();
        Weak { _ptr: self._ptr }
    }
}

impl<T: fmt::Debug> fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(Weak)")
    }
}

#[doc(hidden)]
trait RcBoxPtr<T> {
    fn inner(&self) -> &RcBox<T>;

    #[inline]
    fn strong(&self) -> usize { self.inner().strong.get() }

    #[inline]
    fn inc_strong(&self) { self.inner().strong.set(self.strong() + 1); }

    #[inline]
    fn dec_strong(&self) { self.inner().strong.set(self.strong() - 1); }

    #[inline]
    fn weak(&self) -> usize { self.inner().weak.get() }

    #[inline]
    fn inc_weak(&self) { self.inner().weak.set(self.weak() + 1); }

    #[inline]
    fn dec_weak(&self) { self.inner().weak.set(self.weak() - 1); }
}

impl<T> RcBoxPtr<T> for Rc<T> {
    #[inline(always)]
    fn inner(&self) -> &RcBox<T> {
        unsafe {
            &*self._ptr
        }
    }
}

impl<T> RcBoxPtr<T> for Weak<T> {
    #[inline(always)]
    fn inner(&self) -> &RcBox<T> {
        unsafe {
            &*self._ptr
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Rc, Weak, weak_count, strong_count};
    use std::cell::RefCell;
    use std::mem::drop;

    #[test]
    fn test_clone() {
        let x = Rc::new(RefCell::new(5));
        let y = x.clone();
        *x.borrow_mut() = 20;
        assert_eq!(*y.borrow(), 20);
    }

    #[test]
    fn test_simple() {
        let x = Rc::new(5);
        assert_eq!(*x, 5);
    }

    #[test]
    fn test_simple_clone() {
        let x = Rc::new(5);
        let y = x.clone();
        assert_eq!(*x, 5);
        assert_eq!(*y, 5);
    }

    #[test]
    fn test_destructor() {
        let x: Rc<Box<_>> = Rc::new(Box::new(5));
        assert_eq!(**x, 5);
    }

    #[test]
    fn test_live() {
        let x = Rc::new(5);
        let y = x.downgrade();
        assert!(y.upgrade().is_some());
    }

    #[test]
    fn test_dead() {
        let x = Rc::new(5);
        let y = x.downgrade();
        drop(x);
        assert!(y.upgrade().is_none());
    }

    #[test]
    fn weak_self_cyclic() {
        struct Cycle {
            x: RefCell<Option<Weak<Cycle>>>
        }

        let a = Rc::new(Cycle { x: RefCell::new(None) });
        let b = a.clone().downgrade();
        *a.x.borrow_mut() = Some(b);

        // hopefully we don't double-free (or leak)...
    }

    #[test]
    fn is_unique() {
        let x = Rc::new(3);
        assert!(super::is_unique(&x));
        let y = x.clone();
        assert!(!super::is_unique(&x));
        drop(y);
        assert!(super::is_unique(&x));
        let w = x.downgrade();
        assert!(!super::is_unique(&x));
        drop(w);
        assert!(super::is_unique(&x));
    }

    #[test]
    fn test_strong_count() {
        let a = Rc::new(0u32);
        assert!(strong_count(&a) == 1);
        let w = a.downgrade();
        assert!(strong_count(&a) == 1);
        let b = w.upgrade().expect("upgrade of live rc failed");
        assert!(strong_count(&b) == 2);
        assert!(strong_count(&a) == 2);
        drop(w);
        drop(a);
        assert!(strong_count(&b) == 1);
        let c = b.clone();
        assert!(strong_count(&b) == 2);
        assert!(strong_count(&c) == 2);
    }

    #[test]
    fn test_weak_count() {
        let a = Rc::new(0u32);
        assert!(strong_count(&a) == 1);
        assert!(weak_count(&a) == 0);
        let w = a.downgrade();
        assert!(strong_count(&a) == 1);
        assert!(weak_count(&a) == 1);
        drop(w);
        assert!(strong_count(&a) == 1);
        assert!(weak_count(&a) == 0);
        let c = a.clone();
        assert!(strong_count(&a) == 2);
        assert!(weak_count(&a) == 0);
        drop(c);
    }

    #[test]
    fn try_unwrap() {
        let x = Rc::new(3);
        assert_eq!(super::try_unwrap(x), Ok(3));
        let x = Rc::new(4);
        let _y = x.clone();
        assert_eq!(super::try_unwrap(x), Err(Rc::new(4)));
        let x = Rc::new(5);
        let _w = x.downgrade();
        assert_eq!(super::try_unwrap(x), Err(Rc::new(5)));
    }

    #[test]
    fn get_mut() {
        let mut x = Rc::new(3);
        *super::get_mut(&mut x).unwrap() = 4;
        assert_eq!(*x, 4);
        let y = x.clone();
        assert!(super::get_mut(&mut x).is_none());
        drop(y);
        assert!(super::get_mut(&mut x).is_some());
        let _w = x.downgrade();
        assert!(super::get_mut(&mut x).is_none());
    }

    #[test]
    fn test_cowrc_clone_make_unique() {
        let mut cow0 = Rc::new(75);
        let mut cow1 = cow0.clone();
        let mut cow2 = cow1.clone();

        assert!(75 == *cow0.make_unique());
        assert!(75 == *cow1.make_unique());
        assert!(75 == *cow2.make_unique());

        *cow0.make_unique() += 1;
        *cow1.make_unique() += 2;
        *cow2.make_unique() += 3;

        assert!(76 == *cow0);
        assert!(77 == *cow1);
        assert!(78 == *cow2);

        // none should point to the same backing memory
        assert!(*cow0 != *cow1);
        assert!(*cow0 != *cow2);
        assert!(*cow1 != *cow2);
    }

    #[test]
    fn test_cowrc_clone_unique2() {
        let mut cow0 = Rc::new(75);
        let cow1 = cow0.clone();
        let cow2 = cow1.clone();

        assert!(75 == *cow0);
        assert!(75 == *cow1);
        assert!(75 == *cow2);

        *cow0.make_unique() += 1;

        assert!(76 == *cow0);
        assert!(75 == *cow1);
        assert!(75 == *cow2);

        // cow1 and cow2 should share the same contents
        // cow0 should have a unique reference
        assert!(*cow0 != *cow1);
        assert!(*cow0 != *cow2);
        assert!(*cow1 == *cow2);
    }

    #[test]
    fn test_cowrc_clone_weak() {
        let mut cow0 = Rc::new(75);
        let cow1_weak = cow0.downgrade();

        assert!(75 == *cow0);
        assert!(75 == *cow1_weak.upgrade().unwrap());

        *cow0.make_unique() += 1;

        assert!(76 == *cow0);
        assert!(cow1_weak.upgrade().is_none());
    }

    #[test]
    fn test_show() {
        let foo = Rc::new(75);
        assert_eq!(format!("{:?}", foo), "75");
    }
}
