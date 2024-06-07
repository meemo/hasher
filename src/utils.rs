use std::io;
use std::sync::PoisonError;

use walkdir;
use sqlx;

#[derive(Debug)]
pub enum Error {
    IO,
    Poison,
    Database,
}

impl From<io::Error> for Error {
    fn from(_value: io::Error) -> Self {
        Error::IO
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_value: PoisonError<T>) -> Self {
        Error::Poison
    }
}

impl From<walkdir::Error> for Error {
    fn from(_value: walkdir::Error) -> Self {
        Error::IO
    }
}

impl From<sqlx::Error> for Error {
    fn from(_value: sqlx::Error) -> Self {
        Error::Database
    }
}

#[macro_export]
macro_rules! arclock {
    ($self:ident) => {
        $self.lock().unwrap()
    };
}

#[macro_export]
macro_rules! startthread {
    ($threads:ident, $buffer:ident, $hash_mutex:ident) => {
        let buffer_clone = $buffer.clone();
        let hash_clone = $hash_mutex.clone();

        $threads.push(thread::spawn(move || {
            hash_clone.lock()?.update(buffer_clone.read()?.as_slice());
            Ok(())
        }));
    };
}

#[macro_export]
macro_rules! walkthedir {
    ($path:ident, $args:ident) => {
        WalkDir::new($path)
            .min_depth(0)
            .max_depth($args.max_depth)
            .follow_links(!$args.no_follow_symlinks)
            .contents_first(!$args.breadth_first)
            .sort_by_file_name()
    };
}
