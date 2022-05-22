# `borrowed_with_owner`
A Rust crate that lets you bundle up a borrowed object with its owner. The bundled object has the `'static` lifetime, which means you can store it in a `static`, or pass it to another thread or tokio task, or anything else that requires the object to be `'static`. 

This is inspired by the [`owning_ref` crate](https://docs.rs/owning_ref/latest/owning_ref/), but the borrowed object can be any type that has a `for<'a> BorrowWithLifetime<'a>` impl, whereas `owning_ref` only gives you a few builtin reference types, and requires you to write unsafe code to use it with non-reference types.

Generated docs aren't available since I haven't published this to crates.io yet, but feel free to look at the [source code](./src/lib.rs).
