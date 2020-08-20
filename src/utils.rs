use std::fmt::Display;

pub trait UnwrapWithLog<T> {
    fn unwrap_with_log(self) -> T;
}

pub trait AndThenInto<T, U> {
    fn and_then_into<E: Into<anyhow::Error>, F>
    (self, f: F) -> anyhow::Result<U> where F: FnOnce(T) -> std::result::Result<U, E>;
}

impl<T, E: Into<anyhow::Error>, U> AndThenInto<T, U> for std::result::Result<T, E> {
    fn and_then_into<K: Into<anyhow::Error>, F>
    (self, f: F) -> anyhow::Result<U>
        where F: FnOnce(T) -> std::result::Result<U, K> {
        self.map_err(Into::into)
            .and_then(|x| f(x).map_err(Into::into))
    }
}

impl<T, E: Display> UnwrapWithLog<T> for std::result::Result<T, E> {
    fn unwrap_with_log(self) -> T {
        match self {
            Ok(res) => res,
            Err(e) => {
                log::error!("{}", e);
                std::process::exit(1);
            }
        }
    }
}