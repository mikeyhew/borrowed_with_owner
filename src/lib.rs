/*!
# `borrowed_with_owner`

This crate gives you a way to store borrowed data like `&'a T` or `std::str::Chars<'a>` alongside its owner, giving it a `'static` lifetime. It is inspired by the `owning_ref` crate, but can handle arbitrary borrowed objects without requiring you to write `unsafe` code (and hopefully does not have any soundness issues).

## Why?

As a motivating example, let's say you have a string `s`, and an iterator of the characters within `s` which you get from calling `s.chars()`, and you want to spawn a new thread that does something with them:

```compile_fail
let s: String = "abc".into();
let mut chars = s.chars();

std::thread::spawn(move || {
    assert_eq!(chars.nth(2), Some('c'));
});
```

This example will fail to compile, because the closure we pass to `std::thread::spawn` needs to be `'static`, and `chars` contains a borrow of `s`. Anything borrowing `s` cannot be `'static` because `s` is on the stack and will be dropped when the function returns.

To get around this issue, we could try using a scoped thread API like the one recently introduced to Rust's standard library, currently available in nightly behind a feature flag (you can also use a library like `crossbeam` or `rayon` to get the same functionality in stable Rust):

*/
//! ```
#![cfg_attr(not(feature = "nightly"), doc = "```")]
#![cfg_attr(not(feature = "nightly"), doc = "```ignore")]
//! #![feature(scoped_threads)]
//!
//! let s: String = "abc".into();
//! let mut chars = s.chars();
//!
//! // this function call will block until the closure passed to `scope.spawn()` finishes
//! std::thread::scope(|scope| {
//!     scope.spawn(|| {
//!         assert_eq!(chars.nth(2), Some('c'));
//!     });
//! });
//! ```
/*!
This lets us pass a non-`'static` closure to the `scope.spawn()` call, and ensures that borrowed data on the stack will not be dropped prematurely by blocking the current function until the closure finishes running in the other thread.

However, that may not fit our needs in every case: it may be that we actually do want to let the child thread outlive the current scope; or we could be in the world of `async` Rust where, at the time of writing, there is no suitable way to spawn a scoped task that doesn't block the current thread if the child task needs time to finish. Or it could be that we want to store `chars` in a `static` for some reason. In these cases, we could leak the string so that `chars` can have the `'static` lifetime:

```
let s: String = "abc".into();
let s = Box::leak(s.into_boxed_str());
let mut chars = s.chars();

std::thread::spawn(move || {
    assert_eq!(chars.nth(2), Some('c'));
});
```

This works, but leaks memory: we will never get to reclaim the memory that `s` uses, so we wouldn't want to run this in a loop.

## Enter `borrowed_with_owner`

With this library, however, we can do better: you can bundle up `chars` together with its owner `s` so that, as a whole, the bundled `WithOwner` object fulfills the `'static` requirement. This bundled object can be passed to another thread, and then you can call its `.borrowed_mut()` method to safely get a reference to `chars` that is valid as long as the bundled object is in scope:

```
use borrowed_with_owner::BorrowFromOwner;

struct StdCharsBorrow;

impl<'a> BorrowFromOwner<'a> for StdCharsBorrow {
    type Borrowed = std::str::Chars<'a>;
}

let s: String = "abc".into();
let mut chars_with_s = borrowed_with_owner::RefMutWithOwner::new(s)
    .map::<StdCharsBorrow, _>(|s, _| s.chars());

std::thread::spawn(move || {
    let chars = chars_with_s.borrowed_mut();
    assert_eq!(chars.nth(2), Some('c'));
});
```
*/

use stable_deref_trait::{CloneStableDeref, StableDeref};
use std::ops::{Deref, DerefMut};

/// An immutable (`&T`) reference along with its owner, `O`
pub type RefWithOwner<O> = WithOwner<&'static <O as Deref>::Target, O>;

/// A mutable (`&mut T`) reference along with its owner, `O`
pub type RefMutWithOwner<O> = WithOwner<&'static mut <O as Deref>::Target, O>;

/// A borrowed object held along with its owner, `O`
///
/// Note that `B` isn't necessarily the type of the borrowed object;
/// rather it is just some type that implements `BorrowFromOwner`.
/// `<B as BorrowFromOwner<'a>>::Borrowed` is the type of the borrowed object,
/// where `'a` is the lifetime of the borrow of the `WithOwner` struct
/// when calling the `.borrowed()` or `borrowed_mut()` methods.
pub struct WithOwner<B, O>
where
    B: for<'a> BorrowFromOwner<'a>,
    O: StableDeref,
{
    // `borrowed` is declared first so that it will be dropped first.
    // As stated in the language reference, "The fields of a struct are dropped
    // in declaration order."
    // https://doc.rust-lang.org/reference/destructors.html
    //
    // No code outside of this module should ever see `borrowed` with a `'static`
    // lifetime; this is just used for storage. Instead, `borrowed` can be accessed
    // with the correct lifetime with `.borrowed()`, `.borrowed_mut()`, and `.map(..)`.
    borrowed: <B as BorrowFromOwner<'static>>::Borrowed,
    owner: O,
}

impl<O> RefWithOwner<O>
where
    O: StableDeref,
{
    /// Creates a `RefWithOwner` with `&*owner` as the borrowed value
    pub fn new(owner: O) -> Self {
        // extend the lifetime of &T to &'static T,
        // so we can store it inside of `Self`
        let borrowed = unsafe { &*(&*owner as *const <O as Deref>::Target) };

        Self { owner, borrowed }
    }
}

impl<O> RefMutWithOwner<O>
where
    O: StableDeref + DerefMut,
{
    /// Creates a `RefMutWithOwner` with `&mut *owner` as the borrowed value
    pub fn new(mut owner: O) -> Self {
        // extend the lifetime of &mut T to &'static mut T,
        // so we can store it inside of `Self`
        let borrowed = unsafe { &mut *(&mut *owner as *mut <O as Deref>::Target) };

        Self { owner, borrowed }
    }
}

impl<B, O> WithOwner<B, O>
where
    B: for<'a> BorrowFromOwner<'a>,
    O: StableDeref,
{
    // /// unsound for the same reason as `owner_mut` if `O` has interior mutability.
    // /// just use `Box<RefCell<String>>` instead of `Box<String>` in the below example.
    // pub fn owner(&self) -> &O {
    //     &self.owner
    // }

    // /// unsound because it could invalidate the borrowed value. For example:
    // /// ```
    // /// let mut s = RefWithOwner::new(Box::new(String::from("foo")))
    // ///     .map::<&'static str, _>(|string, _| &*string);
    // ///
    // /// println!("{:?}", s.borrowed());
    // /// // prints `"foo"`
    // ///
    // /// *s.owner_mut() = String::new();
    // ///
    // /// // s.borrowed() now points at freed memory, so this is undefined behaviour
    // /// println!("{:?}", s.borrowed());
    // /// ```
    // pub fn owner_mut(&mut self) -> &mut O {
    //     &mut self.owner
    // }

    /// Drops the borrowed value and returns the owner
    pub fn into_owner(self) -> O {
        self.owner
    }

    /// Returns an `&`-reference to the borrowed value, with lifetime tied to the borrow of `self`
    #[allow(clippy::needless_lifetimes)]
    pub fn borrowed<'a>(&'a self) -> &'a <B as BorrowFromOwner<'a>>::Borrowed {
        unsafe { &*Self::transmute_lifetime_ptr(&self.borrowed as *const _ as *mut _) }
    }

    /// Returns an `&mut`-reference to the borrowed value, with lifetime tied to the borrow of `self`
    #[allow(clippy::needless_lifetimes)]
    pub fn borrowed_mut<'a>(&'a mut self) -> &'a mut <B as BorrowFromOwner<'a>>::Borrowed {
        unsafe { &mut *Self::transmute_lifetime_ptr(&mut self.borrowed) }
    }

    /// Calls `f` with the borrowed value, and returns a new `WithOwner` with the value returned
    /// by `f`. The second `&'a ()` argument to `f` is required because of compiler limitations
    /// and can be ignored.
    pub fn map<B2, F>(self, f: F) -> WithOwner<B2, O>
    where
        B2: for<'a> BorrowFromOwner<'a>,
        F: for<'a> FnOnce(
            <B as BorrowFromOwner<'a>>::Borrowed,
            &'a (), // to get around "lifetime `'a` is unconstrained by the fn input types"
        ) -> <B2 as BorrowFromOwner<'a>>::Borrowed,
    {
        let Self { owner, borrowed } = self;

        let borrowed2 = f(unsafe { Self::transmute_lifetime(borrowed) }, &());

        WithOwner {
            owner,
            borrowed: unsafe { WithOwner::<B2, O>::transmute_lifetime(borrowed2) },
        }
    }

    /// changes the lifetime of a `*mut Borrowed<'a>` to a `*mut Borrowed<'b>`
    unsafe fn transmute_lifetime_ptr<'a, 'b>(
        borrowed: *mut <B as BorrowFromOwner<'a>>::Borrowed,
    ) -> *mut <B as BorrowFromOwner<'b>>::Borrowed {
        // a simple pointer cast, i.e. `borrowed as *mut <B as BorrowFromOwner<'b>>::Borrowed`
        // doesn't work here because Rust complains that the lifetimes aren't the same
        std::mem::transmute(borrowed)
    }

    /// changes the lifetime of a `Borrowed<'a>` to a `Borrowed<'b>`
    unsafe fn transmute_lifetime<'a, 'b>(
        borrowed: <B as BorrowFromOwner<'a>>::Borrowed,
    ) -> <B as BorrowFromOwner<'b>>::Borrowed {
        // `std::mem::transmute` doesn't work here, because it thinks that
        // `borrowed: <B as BorrowFromOwner<'a>>::Borrowed` and
        // `<B as BorrowFromOwner<'b>>::Borrowed` can have different layouts.
        // I believe that they must have the same layout since they only differ by
        // a lifetime (please let me know if this assumption is wrong).
        let transmuted = std::ptr::read(Self::transmute_lifetime_ptr(
            &borrowed as *const _ as *mut _,
        ));
        std::mem::forget(borrowed);
        transmuted
    }
}

impl<B, O> Clone for WithOwner<B, O>
where
    B: for<'a> BorrowFromOwner<'a>,
    for<'a> <B as BorrowFromOwner<'a>>::Borrowed: Clone,
    O: CloneStableDeref,
{
    fn clone(&self) -> Self {
        Self {
            owner: self.owner.clone(),
            borrowed: self.borrowed.clone(),
        }
    }
}

impl<B, O> Copy for WithOwner<B, O>
where
    B: for<'a> BorrowFromOwner<'a>,
    for<'a> <B as BorrowFromOwner<'a>>::Borrowed: Copy,
    O: CloneStableDeref + Copy,
{
}

/// A trait that you implement in order to use a borrowed type with `WithOwner`
///
/// For example, if you have a type `Foo<'a>`, you would implement `for<'a> BorrowFromOwner<'a>`
/// for it like so:
///
/// ```
/// # use borrowed_with_owner::BorrowFromOwner;
/// # struct Foo<'a>(&'a ());
/// impl<'a> BorrowFromOwner<'a> for Foo<'static> {
///     type Borrowed = Foo<'a>;
/// }
/// ```
///
/// Note that the `Self` type (the `Foo<'static>` in this case) of the impl could be any
/// arbitrary type, and doesn't have to be related to the `Borrowed` type used in the impl.
/// However, as a convention, we use the same type as the `Borrowed` type with `'static` used
/// as the `'a` lifetime.
pub trait BorrowFromOwner<'a> {
    type Borrowed: 'a;
}

impl<'a, T: ?Sized> BorrowFromOwner<'a> for &'static T {
    type Borrowed = &'a T;
}

impl<'a, T: ?Sized> BorrowFromOwner<'a> for &'static mut T {
    type Borrowed = &'a mut T;
}

impl<'a> BorrowFromOwner<'a> for () {
    type Borrowed = ();
}
