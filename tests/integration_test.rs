use borrowed_with_owner::{BorrowFromOwner, RefWithOwner};

struct SplitWrapper<'a>(std::str::Split<'a, &'static str>);

impl<'a> BorrowFromOwner<'a> for SplitWrapper<'static> {
    type Borrowed = SplitWrapper<'a>;
}

#[test]
fn it_works() {
    let string: String = "Hello, my name is Michael".into();
    let other_stuff = vec![1, 2, 3];

    let mut string_parts =
        RefWithOwner::new(string).map::<SplitWrapper<'static>, _>(|string, _| {
            dbg!(other_stuff);
            SplitWrapper(string.split(", "))
        });

    std::thread::spawn(move || {
        let parts = string_parts.borrowed_mut().0.clone().collect::<Vec<_>>();

        assert_eq!(parts, ["Hello", "my name is Michael"]);
    })
    .join()
    .unwrap();
}
