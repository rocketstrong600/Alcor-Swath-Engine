use std::ffi::CStr;
use std::panic;

/// Based on take_mut crate
/// Any Type that implements ReplaceWith that is of '&mut Self' can have the value Self Owned, as long as 'Self' is returned afterwards.
/// # Example
/// ```ignore
/// enum Foo {
///   Bar,
///   Baz
/// }
///
/// impl<F> ReplaceWith<F> for Foo {}
///
/// let mut foo = Foo::Bar;
/// let bar: &mut Foo = &mut foo;
/// bar.replace_with(| foo | {
///   drop(foo);
///   Foo::Baz
/// });
/// ```
pub trait ReplaceWith<F> {
    fn replace_with(&mut self, closure: F)
    where
        F: FnOnce(Self) -> Self,
        Self: Sized,
    {
        use std::ptr;

        unsafe {
            let old_t = ptr::read(self);
            let new_t = panic::catch_unwind(panic::AssertUnwindSafe(|| closure(old_t)))
                .unwrap_or_else(|_| ::std::process::abort());
            ptr::write(self, new_t);
        }
    }
}

#[test]
fn replace_with_test() {
    #[derive(PartialEq, Eq, Debug)]
    enum Foo {
        Bar,
        Baz,
    }

    impl<F> ReplaceWith<F> for Foo {}

    let mut foo = Foo::Bar;
    let bar: &mut Foo = &mut foo;
    bar.replace_with(|foo| {
        drop(foo);
        Foo::Baz
    });

    assert_eq!(&foo, &Foo::Baz);
}

#[allow(dead_code)]
pub struct GameInfo {
    pub app_name: &'static CStr,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[allow(dead_code)]
impl Default for GameInfo {
    fn default() -> Self {
        Self {
            app_name: c"",
            major: 0,
            minor: 0,
            patch: 0,
        }
    }
}
