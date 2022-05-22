# `borrowed_with_owner`
A Rust crate that lets you bundle up a borrowed object with its owner. The bundled object has the `'static` lifetime, which means you can store it in a `static`, or pass it between threads, tokio task, or anything else that requires the object to be `'static`.

Generated docs aren't available since I haven't published this to crates.io yet, but feel free to look at the [source code](./src/lib.rs).
