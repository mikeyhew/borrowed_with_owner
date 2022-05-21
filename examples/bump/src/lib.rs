#![cfg(test)]
use borrowed_with_owner::{BorrowWithLifetime, RefWithOwner};
use bumpalo_herd::Herd;
use std::sync::Arc;

struct BorrowSliceOfStrs;

impl<'a> BorrowWithLifetime<'a> for BorrowSliceOfStrs {
    type Borrowed = &'a [&'a str];
}

#[test]
fn test_bumpalo_herd() {
    let herd = RefWithOwner::new(Arc::new(Herd::new()));

    let titles_with_herd = herd.map::<BorrowSliceOfStrs, _>(|herd, _| {
        let member = herd.get();

        let iter = (0..5).map(|i| {
            // note: bumpalo_herd doesn't support bumpalo's `format!` macro yet,
            // so I'm just using the regular `format!` macro for this demo.
            let string = format!("Mambo number {}", i);

            member.alloc_str(&string) as &_
        });

        member.alloc_slice_fill_iter(iter)
    });

    #[allow(clippy::needless_collect)]
    let join_handles = (0..2)
        .map(|_| {
            let titles_with_herd = titles_with_herd.clone();
            std::thread::spawn(move || {
                let slice = titles_with_herd.borrowed();
                assert_eq!(
                    slice,
                    &[
                        "Mambo number 0",
                        "Mambo number 1",
                        "Mambo number 2",
                        "Mambo number 3",
                        "Mambo number 4",
                    ]
                );
            })
        })
        .collect::<Vec<_>>();

    join_handles.into_iter().for_each(|jh| {
        jh.join().unwrap();
    });
}
