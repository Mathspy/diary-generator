#![allow(dead_code)]
mod page;

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs,
    path::Path,
};
use tempdir::TempDir;

pub use page::new as new_page;

#[derive(Debug, PartialEq, Eq)]
pub struct DirEntry {
    name: OsString,
    entry: DirEntryInner,
}

#[derive(Debug, PartialEq, Eq)]
enum DirEntryInner {
    Dir(HashMap<OsString, DirEntryInner>),
    File,
}

impl DirEntry {
    fn into_tuple(self) -> (OsString, DirEntryInner) {
        (self.name, self.entry)
    }

    pub fn dir<T, I>(name: T, entries: I) -> Self
    where
        T: AsRef<OsStr>,
        I: IntoIterator<Item = Self>,
    {
        DirEntry {
            name: name.as_ref().to_owned(),
            entry: DirEntryInner::Dir(entries.into_iter().map(Self::into_tuple).collect()),
        }
    }

    pub fn file<T>(name: T) -> Self
    where
        T: AsRef<OsStr>,
    {
        DirEntry {
            name: name.as_ref().to_owned(),
            entry: DirEntryInner::File,
        }
    }

    pub fn breakdown<P: AsRef<Path>>(path: P) -> Self {
        if !path.as_ref().is_dir() {
            todo!("DirEntry::breakdown currently only handles dir paths");
        }

        let entries = fs::read_dir(path.as_ref())
            .expect("read directory")
            .map(|result| result.expect("read directory files"))
            .map(|dir_entry| {
                (
                    dir_entry.file_name(),
                    dir_entry.file_type().expect("get file type from dir_entry"),
                )
            })
            .map(
                |(file_name, file_type)| match (file_type.is_dir(), file_type.is_file()) {
                    (true, false) => Self::breakdown(path.as_ref().join(&file_name)).into_tuple(),
                    (false, true) => (file_name, DirEntryInner::File),
                    _ => unimplemented!(),
                },
            )
            .collect::<HashMap<_, _>>();

        Self {
            name: path
                .as_ref()
                .file_name()
                .expect("get name of dir in dir_breakdown")
                .to_os_string(),
            entry: DirEntryInner::Dir(entries),
        }
    }
}

pub struct TestDir(TempDir);

impl TestDir {
    pub fn new(function_name: &str) -> Self {
        Self(TempDir::new(&function_name.replace("::", "_")).unwrap())
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

// Adopted from this answer
// https://stackoverflow.com/a/40234666/3018913
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        // Due to tokio::test the actual test will be wrapped in a closure that
        // we want to remove from function name
        const TOKIO_CLOSURE: usize = "::{{closure}}".len();
        &name[..name.len() - 3 - TOKIO_CLOSURE]
    }};
}

pub(crate) use function;
