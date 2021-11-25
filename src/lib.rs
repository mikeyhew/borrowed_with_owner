#![feature(generic_associated_types)]

use stable_deref_trait::StableDeref;
use std::ops::{Deref, DerefMut};

pub trait Borrower {
    type Borrowed<'owner>: 'owner
    where
        Self: 'owner;
}

impl<T> Borrower for &'static T {
    type Borrowed<'owner>
    where
        Self: 'owner,
    = &'owner T;
}

impl<T> Borrower for &'static mut T {
    type Borrowed<'owner>
    where
        Self: 'owner,
    = &'owner mut T;
}

pub struct BorrowedWithOwner<B, O>
where
    B: Borrower + 'static,
    O: StableDeref + 'static,
{
    owner: O,
    borrowed: <B as Borrower>::Borrowed<'static>,
}

impl<B, O> BorrowedWithOwner<B, O>
where
    B: Borrower + 'static,
    O: StableDeref + 'static,
{
    pub fn new_fn(
        owner: O,
        borrow_fn: for<'owner> fn(
            &'owner <O as Deref>::Target,
        ) -> <B as Borrower>::Borrowed<'owner>,
    ) -> Self {
        let borrowed = borrow_fn(&*owner);
        let borrowed = unsafe { Self::transmute_lifetime(borrowed) };
        Self { owner, borrowed }
    }

    pub fn new_mut_fn(
        mut owner: O,
        borrow_fn: for<'owner> fn(
            &'owner mut <O as Deref>::Target,
        ) -> <B as Borrower>::Borrowed<'owner>,
    ) -> Self
    where
        O: StableDeref + DerefMut,
    {
        let borrowed = borrow_fn(&mut owner);
        let borrowed = unsafe { Self::transmute_lifetime(borrowed) };
        Self { owner, borrowed }
    }

    // // Rust doesn't like this type signature, says the 'owner lifetime
    // // in the return type is unconstrained by the input types
    // pub fn map<B2>(
    //     self,
    //     map_fn: for<'owner> fn(
    //         <B as Borrower>::Borrowed<'owner>,
    //     ) -> <B2 as Borrower>::Borrowed<'owner>,
    // ) -> BorrowedWithOwner<B2, O>
    // where
    //     B2: Borrower,
    // {
    //     todo!()
    // }

    pub fn owner(&self) -> &O {
        &self.owner
    }

    pub fn owner_mut(&mut self) -> &mut O {
        &mut self.owner
    }

    pub fn into_owner(self) -> O {
        self.owner
    }

    pub fn borrowed<'a>(&'a self) -> &'a <B as Borrower>::Borrowed<'a> {
        unsafe { &*Self::transmute_lifetime_ptr(&self.borrowed as *const _ as *mut _) }
    }

    pub fn borrowed_mut<'a>(&'a mut self) -> &'a mut <B as Borrower>::Borrowed<'a> {
        unsafe { &mut *Self::transmute_lifetime_ptr(&mut self.borrowed) }
    }

    /// changes the lifetime of a `*mut Borrowed<'a>` to a `*mut Borrowed<'b>`,
    /// which Rust won't let you do with simple pointer casts
    unsafe fn transmute_lifetime_ptr<'a, 'b>(
        borrowed: *mut <B as Borrower>::Borrowed<'a>,
    ) -> *mut <B as Borrower>::Borrowed<'b> {
        std::mem::transmute_copy(&borrowed)
    }

    /// changes the lifetime of a `Borrowed<'a>` to a `Borrowed<'b>`
    /// which Rust won't let you do with `std::mem::transmute`
    /// (I guess it thinks the layout of `Borrowed` could
    /// change depending on the lifetime)
    unsafe fn transmute_lifetime<'a, 'b>(
        borrowed: <B as Borrower>::Borrowed<'a>,
    ) -> <B as Borrower>::Borrowed<'b> {
        let transmuted = std::ptr::read(Self::transmute_lifetime_ptr(
            &borrowed as *const _ as *mut _,
        ));
        std::mem::forget(borrowed);
        transmuted
    }
}
