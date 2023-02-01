//!
//! [<img alt="github" src="https://img.shields.io/badge/github-workflow--rs-8da0cb?style=for-the-badge&labelColor=555555&color=8da0cb&logo=github" height="20">](https://github.com/workflow-rs/workflow-rs)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/workflow-store.svg?maxAge=2592000&style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/workflow-store)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-workflow--store-56c2a5?maxAge=2592000&style=for-the-badge&logo=rust" height="20">](https://docs.rs/workflow-store)
//! <img alt="license" src="https://img.shields.io/crates/l/workflow-store.svg?maxAge=2592000&color=6ac&style=for-the-badge&logoColor=fff" height="20">
//! <img src="https://img.shields.io/badge/platform- wasm32/browser -informational?style=for-the-badge&color=50a0f0" height="20">
//! <img src="https://img.shields.io/badge/platform- wasm32/node.js -informational?style=for-the-badge&color=50a0f0" height="20">
//!
//! This crate provides an abstraction layer for storing and loading
//! data in different environments: File I/O on desktop devices and
//! local storage when running in the browser.  The goal behind this
//! crate is to allow for a single initialization-phase configuration,
//! following which the API can be used throughout the application
//! without the concern about the operating environment.
//!
//!

pub mod error;
pub mod result;

use crate::result::Result;
use cfg_if::cfg_if;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use async_std::path::PathBuf;
        use async_std::fs;
    } else {
        use base64::{encode, decode};
    }
}

///
/// # Store
///
/// A simple file loader that allows user to
/// specify different paths on various
/// operating systems with fallbacks.
///
pub struct Store {
    // linux (fallsback to unix, generic)
    pub linux: Option<String>,
    // macos (fallsback to unix, generic)
    pub macos: Option<String>,
    // unix (fallsback to generic)
    pub unix: Option<String>,
    // windows (fallsback to generic)
    pub windows: Option<String>,
    // fallback for all OSes
    pub generic: Option<String>,
    // browser locastorage (fallsback to a hash of generic in hex)
    pub browser: Option<String>,
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    pub fn new() -> Store {
        Store {
            linux: None,
            macos: None,
            unix: None,
            windows: None,
            generic: None,
            browser: None,
        }
    }

    pub fn with_linux(&mut self, linux: &str) -> &mut Store {
        self.linux = Some(linux.to_string());
        self
    }

    pub fn with_macos(&mut self, macos: &str) -> &mut Store {
        self.macos = Some(macos.to_string());
        self
    }

    pub fn with_unix(&mut self, unix: &str) -> &mut Store {
        self.unix = Some(unix.to_string());
        self
    }

    pub fn with_windows(&mut self, windows: &str) -> &mut Store {
        self.windows = Some(windows.to_string());
        self
    }

    pub fn with_generic(&mut self, generic: &str) -> &mut Store {
        self.generic = Some(generic.to_string());
        self
    }

    pub fn with_browser(&mut self, browser: &str) -> &mut Store {
        self.browser = Some(browser.to_string());
        self
    }

    pub fn filename(&self) -> String {
        cfg_if! {
            if #[cfg(target_os = "macos")] {
                find(&[self.macos.as_ref(),self.unix.as_ref(),self.generic.as_ref()])
            } else if #[cfg(target_os = "linux")] {
                find(&[self.linux.as_ref(),self.unix.as_ref(),self.generic.as_ref()])
            } else if #[cfg(target_family = "unix")] {
                find(&[self.unix.as_ref(),self.generic.as_ref()])
            } else if #[cfg(target_family = "windows")] {
                find(&[self.windows.as_ref(),self.generic.as_ref()])
            } else if #[cfg(target_arch = "wasm32")] {
                if let Some(browser) = self.browser.as_ref() {
                    browser.clone()
                } else if let Some(generic) = self.generic.as_ref() {
                    // hash of generic
                    hash(generic)
                } else {
                    panic!("no path found for the current operating environment");
                }
            }
        }
    }

    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            pub async fn exists(&self) -> Result<bool> {
                let filename = self.filename();
                Ok(local_storage().get_item(&filename)?.is_some())
            }

            pub async fn read(&self) -> Result<Vec<u8>> {
                let filename = self.filename();
                let v = local_storage().get_item(&filename)?.unwrap();
                Ok(decode(v)?)
            }

            pub async fn write(&self, data: &[u8]) -> Result<()> {
                let filename = self.filename();
                let v = encode(data);
                local_storage().set_item(&filename, &v)?;
                Ok(())
            }

        } else {
            pub async fn exists(&self) -> Result<bool> {
                let filename = parse(self.filename());
                Ok(filename.exists().await)
            }

            pub async fn read(&self) -> Result<Vec<u8>> {
                let filename = parse(self.filename());
                Ok(fs::read(&filename).await?)
            }

            pub async fn write(&self, data: &[u8]) -> Result<()> {
                let filename = parse(self.filename());
                Ok(fs::write(&filename, data).await?)
            }
        }
    }
}

cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        pub fn parse(path : String) -> PathBuf {

            if let Some(stripped) = path.strip_prefix('~') {
                let home_dir: PathBuf = home::home_dir().unwrap().into();
                home_dir.join(stripped)
            } else {
                PathBuf::from(path)
            }
        }
    } else {
        pub fn local_storage() -> web_sys::Storage {
            web_sys::window().unwrap().local_storage().unwrap().unwrap()
        }
    }
}

pub fn find(paths: &[Option<&String>]) -> String {
    for path in paths.iter() {
        if let Some(path) = *path {
            return path.clone();
        }
    }
    panic!("no path found for the current operating environment");
}

pub fn hash<T>(t: T) -> String
where
    T: Hash,
{
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    let v = hasher.finish();
    format!("{v:x}")
}
