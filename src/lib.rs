// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Thread-local reference-counted boxes (the `Rc<T>` type).
//!
//! The `Rc<T>` type provides shared ownership of an immutable value.
//! Destruction is deterministic, and will occur as soon as the last owner is
//! gone. It is marked as non-sendable because it avoids the overhead of atomic
//! reference counting.
//!
//! The `downgrade` method can be used to create a non-owning `Weak<T>` pointer
//! to the box. A `Weak<T>` pointer can be upgraded to an `Rc<T>` pointer, but
//! will return `None` if the value has already been dropped.
//!
//! For example, a tree with parent pointers can be represented by putting the
//! nodes behind strong `Rc<T>` pointers, and then storing the parent pointers
//! as `Weak<T>` pointers.
//!
//! # Examples
//!
//! Consider a scenario where a set of `Gadget`s are owned by a given `Owner`.
//! We want to have our `Gadget`s point to their `Owner`. We can't do this with
//! unique ownership, because more than one gadget may belong to the same
//! `Owner`. `Rc<T>` allows us to share an `Owner` between multiple `Gadget`s,
//! and have the `Owner` remain allocated as long as any `Gadget` points at it.
//!
//! ```rust
//! use rc::Rc;
//!
//! struct Owner {
//!     name: String
//!     // ...other fields
//! }
//!
//! struct Gadget {
//!     id: i32,
//!     owner: Rc<Owner>
//!     // ...other fields
//! }
//!
//! fn main() {
//!     // Create a reference counted Owner.
//!     let gadget_owner : Rc<Owner> = Rc::new(
//!             Owner { name: String::from("Gadget Man") }
//!     );
//!
//!     // Create Gadgets belonging to gadget_owner. To increment the reference
//!     // count we clone the `Rc<T>` object.
//!     let gadget1 = Gadget { id: 1, owner: gadget_owner.clone() };
//!     let gadget2 = Gadget { id: 2, owner: gadget_owner.clone() };
//!
//!     drop(gadget_owner);
//!
//!     // Despite dropping gadget_owner, we're still able to print out the name
//!     // of the Owner of the Gadgets. This is because we've only dropped the
//!     // reference count object, not the Owner it wraps. As long as there are
//!     // other `Rc<T>` objects pointing at the same Owner, it will remain
//!     // allocated. Notice that the `Rc<T>` wrapper around Gadget.owner gets
//!     // automatically dereferenced for us.
//!     println!("Gadget {} owned by {}", gadget1.id, gadget1.owner.name);
//!     println!("Gadget {} owned by {}", gadget2.id, gadget2.owner.name);
//!
//!     // At the end of the method, gadget1 and gadget2 get destroyed, and with
//!     // them the last counted references to our Owner. Gadget Man now gets
//!     // destroyed as well.
//! }
//! ```
//!
//! If our requirements change, and we also need to be able to traverse from
//! Owner → Gadget, we will run into problems: an `Rc<T>` pointer from Owner
//! → Gadget introduces a cycle between the objects. This means that their
//! reference counts can never reach 0, and the objects will remain allocated: a
//! memory leak. In order to get around this, we can use `Weak<T>` pointers.
//! These pointers don't contribute to the total count.
//!
//! Rust actually makes it somewhat difficult to produce this loop in the first
//! place: in order to end up with two objects that point at each other, one of
//! them needs to be mutable. This is problematic because `Rc<T>` enforces
//! memory safety by only giving out shared references to the object it wraps,
//! and these don't allow direct mutation. We need to wrap the part of the
//! object we wish to mutate in a `RefCell`, which provides *interior
//! mutability*: a method to achieve mutability through a shared reference.
//! `RefCell` enforces Rust's borrowing rules at runtime.  Read the `Cell`
//! documentation for more details on interior mutability.
//!
//! ```rust
#![cfg_attr(feature = "unstable", doc = "# #![feature(rc_weak)]")]
//! use rc::Rc;
//! use rc::Weak;
//! use std::cell::RefCell;
//!
//! struct Owner {
//!     name: String,
//!     gadgets: RefCell<Vec<Weak<Gadget>>>
//!     // ...other fields
//! }
//!
//! struct Gadget {
//!     id: i32,
//!     owner: Rc<Owner>
//!     // ...other fields
//! }
//!
//! fn main() {
//!     // Create a reference counted Owner. Note the fact that we've put the
//!     // Owner's vector of Gadgets inside a RefCell so that we can mutate it
//!     // through a shared reference.
//!     let gadget_owner : Rc<Owner> = Rc::new(
//!             Owner {
//!                 name: "Gadget Man".to_string(),
//!                 gadgets: RefCell::new(Vec::new())
//!             }
//!     );
//!
//!     // Create Gadgets belonging to gadget_owner as before.
//!     let gadget1 = Rc::new(Gadget{id: 1, owner: gadget_owner.clone()});
//!     let gadget2 = Rc::new(Gadget{id: 2, owner: gadget_owner.clone()});
//!
//!     // Add the Gadgets to their Owner. To do this we mutably borrow from
//!     // the RefCell holding the Owner's Gadgets.
//!     gadget_owner.gadgets.borrow_mut().push(gadget1.clone().downgrade());
//!     gadget_owner.gadgets.borrow_mut().push(gadget2.clone().downgrade());
//!
//!     // Iterate over our Gadgets, printing their details out
//!     for gadget_opt in gadget_owner.gadgets.borrow().iter() {
//!
//!         // gadget_opt is a Weak<Gadget>. Since weak pointers can't guarantee
//!         // that their object is still allocated, we need to call upgrade()
//!         // on them to turn them into a strong reference. This returns an
//!         // Option, which contains a reference to our object if it still
//!         // exists.
//!         let gadget = gadget_opt.upgrade().unwrap();
//!         println!("Gadget {} owned by {}", gadget.id, gadget.owner.name);
//!     }
//!
//!     // At the end of the method, gadget_owner, gadget1 and gadget2 get
//!     // destroyed. There are now no strong (`Rc<T>`) references to the gadgets.
//!     // Once they get destroyed, the Gadgets get destroyed. This zeroes the
//!     // reference count on Gadget Man, they get destroyed as well.
//! }
//! ```

#[cfg(feature = "unstable")] pub use std::rc::*;
#[cfg(not(feature = "unstable"))] pub use stable_rc::*;
#[cfg(not(feature = "unstable"))] mod stable_rc;
