pub mod lorawan;
pub mod snap;

enum MyOption<T> {
    Some(T),
    None,
}

impl<T> From<MyOption<T>> for Option<T> {
    fn from(value: MyOption<T>) -> Self {
        match value {
            MyOption::Some(val) => Some(val),
            MyOption::None => None,
        }
    }
}
