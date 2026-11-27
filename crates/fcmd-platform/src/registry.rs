//! The Windows registry: its keys listed, its values read and written (SPEC 06 §17).
//!
//! A key is named the way regedit names it, root first: `HKEY_CURRENT_USER\Software\Vendor`.
//! Everything goes through the [`Registry`] trait, which has two implementations: the
//! system's ([`SystemRegistry`], the Win32 calls on Windows and *not on this system*
//! everywhere else) and one kept in memory ([`MemoryRegistry`]), which is what the tests of
//! this crate and of the program run against — the panel's logic, the backup made before a
//! change and the refusal when there is none are all tested on machines that have no
//! registry at all.
//!
//! What is built on the trait rather than in it is here as well, written once for both:
//! a subtree read into the blocks of a `.reg` file ([`export`]), such blocks written into
//! the registry ([`apply`]), a count of what a key holds, and a value renamed.
//!
//! Nothing here asks for elevation or runs as anybody else: a key this user may not write
//! answers [`RegError::Denied`], and the caller says so.

use crate::regfile::{KeyBlock, Line, Value};

/// The five roots, in regedit's order.
pub const ROOTS: [&str; 5] = [
    "HKEY_CLASSES_ROOT",
    "HKEY_CURRENT_USER",
    "HKEY_LOCAL_MACHINE",
    "HKEY_USERS",
    "HKEY_CURRENT_CONFIG",
];

/// Why the registry did not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegError {
    /// There is no registry on this system.
    Unsupported,
    /// The key is not this user's to read or to write — `HKEY_LOCAL_MACHINE` without
    /// elevation, typically.
    Denied,
    /// No such key or value.
    NotFound,
    /// Anything else, in the system's own words.
    Failed(String),
}

/// A registry: the system's, or one kept in memory.
///
/// Names are compared the way the registry compares them, without regard to case, and a
/// value's empty name is the key's default value.
pub trait Registry: Send {
    /// The names of the keys directly under this one.
    fn subkeys(&self, key: &str) -> Result<Vec<String>, RegError>;
    /// The values of this key, with their bytes.
    fn values(&self, key: &str) -> Result<Vec<Value>, RegError>;
    /// Write one value, replacing the one of that name.
    fn set_value(&self, key: &str, value: &Value) -> Result<(), RegError>;
    fn delete_value(&self, key: &str, name: &str) -> Result<(), RegError>;
    /// Make the key, and the keys above it that are missing. `false` when it was there.
    fn create_key(&self, key: &str) -> Result<bool, RegError>;
    /// Remove the key and everything under it.
    fn delete_tree(&self, key: &str) -> Result<(), RegError>;
    /// Give the key another name, where it is.
    fn rename_key(&self, key: &str, name: &str) -> Result<(), RegError>;
}

/// The key above this one, and `None` for a root.
pub fn parent(key: &str) -> Option<&str> {
    key.rsplit_once('\\').map(|(above, _)| above)
}

/// The key's own name: what is after the last `\`.
pub fn leaf(key: &str) -> &str {
    key.rsplit_once('\\').map_or(key, |(_, name)| name)
}

/// The key of this name under `key`.
pub fn join(key: &str, name: &str) -> String {
    format!("{key}\\{name}")
}

/// Whether two names are one to the registry.
pub fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// How many keys there are under this one, all the way down.
pub fn count_subkeys(reg: &dyn Registry, key: &str) -> Result<usize, RegError> {
    let mut count = 0;
    for name in reg.subkeys(key)? {
        count += 1;
        // A key that cannot be looked into still counts as the one key it is.
        count += count_subkeys(reg, &join(key, &name)).unwrap_or(0);
    }
    Ok(count)
}

/// The key as the blocks of a `.reg` file: itself and its values, and with `deep` every key
/// under it, each after the one that holds it — the order regedit writes and the one an
/// import needs.
///
/// A key under the first that cannot be read fails the whole export: a backup with a hole
/// in it is not one.
pub fn export(reg: &dyn Registry, key: &str, deep: bool) -> Result<Vec<KeyBlock>, RegError> {
    let mut out = Vec::new();
    export_into(reg, key, deep, &mut out)?;
    Ok(out)
}

fn export_into(
    reg: &dyn Registry,
    key: &str,
    deep: bool,
    out: &mut Vec<KeyBlock>,
) -> Result<(), RegError> {
    let mut values = reg.values(key)?;
    // The default value first, then by name: the same file for the same key every time.
    values.sort_by_key(|v| v.name.to_lowercase());
    out.push(KeyBlock {
        path: key.to_string(),
        delete: false,
        lines: values.into_iter().map(Line::Set).collect(),
    });
    if deep {
        let mut names = reg.subkeys(key)?;
        names.sort_by_key(|n| n.to_lowercase());
        for name in names {
            export_into(reg, &join(key, &name), true, out)?;
        }
    }
    Ok(())
}

/// Write the blocks of a `.reg` file into the registry, in the file's order: a `[-key]`
/// removes the key and what is under it, any other key is made if it is missing and has its
/// values written or taken away. Stops at the first refusal.
pub fn apply(reg: &dyn Registry, blocks: &[KeyBlock]) -> Result<(), RegError> {
    for block in blocks {
        if block.delete {
            match reg.delete_tree(&block.path) {
                // Asked to be gone and already gone.
                Ok(()) | Err(RegError::NotFound) => {}
                Err(why) => return Err(why),
            }
            continue;
        }
        reg.create_key(&block.path)?;
        for line in &block.lines {
            match line {
                Line::Set(value) => reg.set_value(&block.path, value)?,
                Line::Delete(name) => match reg.delete_value(&block.path, name) {
                    Ok(()) | Err(RegError::NotFound) => {}
                    Err(why) => return Err(why),
                },
            }
        }
    }
    Ok(())
}

/// Give a value another name: written under the new one, then taken away under the old —
/// in that order, so that a refusal half way leaves the value under one name at least.
pub fn rename_value(reg: &dyn Registry, key: &str, old: &str, new: &str) -> Result<(), RegError> {
    let values = reg.values(key)?;
    let mut value = values
        .into_iter()
        .find(|v| same_name(&v.name, old))
        .ok_or(RegError::NotFound)?;
    value.name = new.to_string();
    reg.set_value(key, &value)?;
    // Another case of the same name is the same value, already rewritten.
    if !same_name(old, new) {
        reg.delete_value(key, old)?;
    }
    Ok(())
}

// ── the system's ────────────────────────────────────────────────────────────────────────

/// The registry of the machine the program runs on. Everywhere but Windows every call
/// answers [`RegError::Unsupported`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRegistry;

impl SystemRegistry {
    /// Whether this system has a registry at all: the drive menu offers the panel only
    /// where it does.
    pub const AVAILABLE: bool = cfg!(windows);
}

#[cfg(not(windows))]
impl Registry for SystemRegistry {
    fn subkeys(&self, _key: &str) -> Result<Vec<String>, RegError> {
        Err(RegError::Unsupported)
    }
    fn values(&self, _key: &str) -> Result<Vec<Value>, RegError> {
        Err(RegError::Unsupported)
    }
    fn set_value(&self, _key: &str, _value: &Value) -> Result<(), RegError> {
        Err(RegError::Unsupported)
    }
    fn delete_value(&self, _key: &str, _name: &str) -> Result<(), RegError> {
        Err(RegError::Unsupported)
    }
    fn create_key(&self, _key: &str) -> Result<bool, RegError> {
        Err(RegError::Unsupported)
    }
    fn delete_tree(&self, _key: &str) -> Result<(), RegError> {
        Err(RegError::Unsupported)
    }
    fn rename_key(&self, _key: &str, _name: &str) -> Result<(), RegError> {
        Err(RegError::Unsupported)
    }
}

#[cfg(windows)]
impl Registry for SystemRegistry {
    fn subkeys(&self, key: &str) -> Result<Vec<String>, RegError> {
        sys::subkeys(key)
    }
    fn values(&self, key: &str) -> Result<Vec<Value>, RegError> {
        sys::values(key)
    }
    fn set_value(&self, key: &str, value: &Value) -> Result<(), RegError> {
        sys::set_value(key, value)
    }
    fn delete_value(&self, key: &str, name: &str) -> Result<(), RegError> {
        sys::delete_value(key, name)
    }
    fn create_key(&self, key: &str) -> Result<bool, RegError> {
        sys::create_key(key)
    }
    fn delete_tree(&self, key: &str) -> Result<(), RegError> {
        sys::delete_tree(key)
    }
    fn rename_key(&self, key: &str, name: &str) -> Result<(), RegError> {
        sys::rename_key(key, name)
    }
}

#[cfg(windows)]
mod sys {
    use super::{RegError, Value};
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS,
        ERROR_PATH_NOT_FOUND, ERROR_SUCCESS,
    };
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegDeleteTreeW, RegDeleteValueW,
        RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryInfoKeyW, RegRenameKey,
        RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER,
        HKEY_LOCAL_MACHINE, HKEY_USERS, KEY_ENUMERATE_SUB_KEYS, KEY_QUERY_VALUE, KEY_READ,
        KEY_SET_VALUE, KEY_WRITE, REG_CREATED_NEW_KEY, REG_OPTION_NON_VOLATILE,
    };

    /// The standard right to delete an object, which a key must be opened with before its
    /// subtree can be emptied (`DELETE` in `winnt.h`).
    const DELETE: u32 = 0x0001_0000;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn error(code: u32) -> RegError {
        match code {
            ERROR_ACCESS_DENIED => RegError::Denied,
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => RegError::NotFound,
            // The system's own sentence for the code, which `std` already knows how to ask
            // for.
            other => RegError::Failed(std::io::Error::from_raw_os_error(other as i32).to_string()),
        }
    }

    fn check(code: u32) -> Result<(), RegError> {
        if code == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(error(code))
        }
    }

    /// The root a key's name starts with, and the rest of the name.
    fn split(key: &str) -> Result<(HKEY, &str), RegError> {
        let (root, rest) = key.split_once('\\').unwrap_or((key, ""));
        let root = match root.to_ascii_uppercase().as_str() {
            "HKEY_CLASSES_ROOT" => HKEY_CLASSES_ROOT,
            "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
            "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
            "HKEY_USERS" => HKEY_USERS,
            "HKEY_CURRENT_CONFIG" => HKEY_CURRENT_CONFIG,
            _ => return Err(RegError::NotFound),
        };
        Ok((root, rest))
    }

    /// An open key, closed when it goes out of scope.
    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: the handle came from `RegOpenKeyExW` or `RegCreateKeyExW`, which
            // succeeded, and is closed exactly once, here.
            unsafe { RegCloseKey(self.0) };
        }
    }

    /// The key, opened with these rights. A root is opened like any other key — through an
    /// empty name under itself — so that every handle here is one to close.
    fn open(key: &str, access: u32) -> Result<Key, RegError> {
        let (root, rest) = split(key)?;
        let path = wide(rest);
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: `root` is one of the predefined handles, `path` is a NUL-terminated
        // UTF-16 string that outlives the call, and `handle` is a valid place for the
        // result, read only when the call says it succeeded.
        check(unsafe { RegOpenKeyExW(root, path.as_ptr(), 0, access, &mut handle) })?;
        Ok(Key(handle))
    }

    pub fn subkeys(key: &str) -> Result<Vec<String>, RegError> {
        let key = open(key, KEY_READ)?;
        let mut out = Vec::new();
        // A key's name is at most 255 characters; the buffer has room for its terminator.
        let mut name = [0u16; 256];
        for index in 0.. {
            let mut len = name.len() as u32;
            // SAFETY: `key.0` is open with `KEY_ENUMERATE_SUB_KEYS` (part of `KEY_READ`);
            // `name` is a writable buffer of `len` characters; the optional pointers are
            // null, which the call allows.
            let code = unsafe {
                RegEnumKeyExW(
                    key.0,
                    index,
                    name.as_mut_ptr(),
                    &mut len,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if code == ERROR_NO_MORE_ITEMS {
                break;
            }
            check(code)?;
            out.push(String::from_utf16_lossy(&name[..len as usize]));
        }
        Ok(out)
    }

    pub fn values(key: &str) -> Result<Vec<Value>, RegError> {
        let key = open(key, KEY_READ)?;
        let mut longest: u32 = 0;
        // SAFETY: `key.0` is open with `KEY_QUERY_VALUE` (part of `KEY_READ`); the one
        // answer asked for goes to a valid `u32`, and every other pointer is null, which
        // the call allows for what the caller does not want.
        check(unsafe {
            RegQueryInfoKeyW(
                key.0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut longest,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        })?;
        let mut out = Vec::new();
        // A value's name is at most 16 383 characters.
        let mut name = vec![0u16; 16_384];
        let mut data = vec![0u8; longest.max(64) as usize];
        let mut index = 0;
        loop {
            let mut name_len = name.len() as u32;
            let mut data_len = data.len() as u32;
            let mut kind: u32 = 0;
            // SAFETY: `key.0` is open with `KEY_QUERY_VALUE`; `name` and `data` are
            // writable buffers of the lengths passed beside them, and `kind` is a valid
            // `u32`. The lengths are read back only after the call returns.
            let code = unsafe {
                RegEnumValueW(
                    key.0,
                    index,
                    name.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null(),
                    &mut kind,
                    data.as_mut_ptr(),
                    &mut data_len,
                )
            };
            if code == ERROR_NO_MORE_ITEMS {
                break;
            }
            // Somebody wrote a longer value since the key was asked how long they are: the
            // call says how much room this one wants, and the same index is asked again.
            if code == ERROR_MORE_DATA && data_len as usize > data.len() {
                data.resize(data_len as usize, 0);
                continue;
            }
            check(code)?;
            out.push(Value {
                name: String::from_utf16_lossy(&name[..name_len as usize]),
                kind,
                data: data[..data_len as usize].to_vec(),
            });
            index += 1;
        }
        Ok(out)
    }

    pub fn set_value(key: &str, value: &Value) -> Result<(), RegError> {
        let key = open(key, KEY_SET_VALUE)?;
        let name = wide(&value.name);
        let len = u32::try_from(value.data.len())
            .map_err(|_| RegError::Failed("the value is too long".into()))?;
        // SAFETY: `key.0` is open with `KEY_SET_VALUE`; `name` is NUL-terminated and
        // outlives the call; `data` points at `len` readable bytes (a dangling but aligned
        // pointer when `len` is zero, which is never read).
        check(unsafe {
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                value.kind,
                value.data.as_ptr(),
                len,
            )
        })
    }

    pub fn delete_value(key: &str, name: &str) -> Result<(), RegError> {
        let key = open(key, KEY_SET_VALUE)?;
        let name = wide(name);
        // SAFETY: `key.0` is open with `KEY_SET_VALUE` and `name` is a NUL-terminated
        // string that outlives the call.
        check(unsafe { RegDeleteValueW(key.0, name.as_ptr()) })
    }

    pub fn create_key(key: &str) -> Result<bool, RegError> {
        let (root, rest) = split(key)?;
        if rest.is_empty() {
            return Ok(false);
        }
        let path = wide(rest);
        let mut handle: HKEY = std::ptr::null_mut();
        let mut made: u32 = 0;
        // SAFETY: `root` is a predefined handle, `path` a NUL-terminated string that
        // outlives the call; the class and the security attributes are null, which asks for
        // the defaults; `handle` and `made` are valid places for the two answers.
        check(unsafe {
            RegCreateKeyExW(
                root,
                path.as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_READ | KEY_WRITE,
                std::ptr::null(),
                &mut handle,
                &mut made,
            )
        })?;
        drop(Key(handle));
        Ok(made == REG_CREATED_NEW_KEY)
    }

    pub fn delete_tree(key: &str) -> Result<(), RegError> {
        let (root, rest) = split(key)?;
        if rest.is_empty() {
            // A root is not anybody's to delete.
            return Err(RegError::Denied);
        }
        {
            let key = open(
                key,
                DELETE | KEY_ENUMERATE_SUB_KEYS | KEY_QUERY_VALUE | KEY_SET_VALUE,
            )?;
            // SAFETY: `key.0` is open with the rights the call asks for; a null name means
            // "what is under this key", which is what is to be emptied.
            check(unsafe { RegDeleteTreeW(key.0, std::ptr::null()) })?;
        }
        // And the key itself, now empty, by its name under its root: the rights of the
        // handle this is asked through do not matter to the call, only the key's own.
        let path = wide(rest);
        // SAFETY: `root` is a predefined handle and `path` a NUL-terminated string that
        // outlives the call.
        check(unsafe { RegDeleteKeyW(root, path.as_ptr()) })
    }

    pub fn rename_key(key: &str, name: &str) -> Result<(), RegError> {
        let (root, rest) = split(key)?;
        if rest.is_empty() {
            return Err(RegError::Denied);
        }
        let path = wide(rest);
        let name = wide(name);
        // SAFETY: `root` is a predefined handle; both strings are NUL-terminated and
        // outlive the call.
        check(unsafe { RegRenameKey(root, path.as_ptr(), name.as_ptr()) })
    }
}

// ── one kept in memory ──────────────────────────────────────────────────────────────────

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// A registry kept in memory, with the five roots in it and nothing else.
///
/// It is what the tests run against, here and in the program, and it is shared: a clone is
/// the same registry, so a test keeps one hand on it while the program holds the other. A
/// key can be closed to writing ([`MemoryRegistry::deny`]) to stand for `HKEY_LOCAL_MACHINE`
/// in front of somebody who is not an administrator.
#[derive(Debug, Clone)]
pub struct MemoryRegistry {
    inner: Arc<Mutex<Memory>>,
}

#[derive(Debug, Default)]
struct Memory {
    /// Each key under its name in lower case, which is how the registry compares names:
    /// the name as it was written, and the values.
    keys: BTreeMap<String, (String, Vec<Value>)>,
    /// Keys nobody may write, in lower case: they and everything under them.
    denied: Vec<String>,
    /// Every change that was made, in order, for a test to read.
    log: Vec<String>,
}

impl Default for MemoryRegistry {
    fn default() -> Self {
        MemoryRegistry::new()
    }
}

impl MemoryRegistry {
    pub fn new() -> MemoryRegistry {
        let mut memory = Memory::default();
        for root in ROOTS {
            memory
                .keys
                .insert(root.to_lowercase(), (root.to_string(), Vec::new()));
        }
        MemoryRegistry {
            inner: Arc::new(Mutex::new(memory)),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Memory> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A key and these values in it, made without leaving a trace in the log: what a test
    /// starts from.
    pub fn with(self, key: &str, values: Vec<Value>) -> MemoryRegistry {
        {
            let mut memory = self.lock();
            memory.make(key);
            if let Some(entry) = memory.keys.get_mut(&key.to_lowercase()) {
                entry.1 = values;
            }
        }
        self
    }

    /// From now on this key and everything under it refuses to be written.
    pub fn deny(&self, key: &str) {
        self.lock().denied.push(key.to_lowercase());
    }

    /// The changes made so far, in order: `set key\name`, `delete-value key\name`,
    /// `create key`, `delete-tree key`, `rename key -> name`.
    pub fn log(&self) -> Vec<String> {
        self.lock().log.clone()
    }

    /// Whether there is such a key.
    pub fn has_key(&self, key: &str) -> bool {
        self.lock().keys.contains_key(&key.to_lowercase())
    }

    /// One value of a key, if both are there.
    pub fn value(&self, key: &str, name: &str) -> Option<Value> {
        let memory = self.lock();
        let (_, values) = memory.keys.get(&key.to_lowercase())?;
        values.iter().find(|v| same_name(&v.name, name)).cloned()
    }
}

impl Memory {
    /// The key and every key above it that is missing.
    fn make(&mut self, key: &str) {
        let mut path = String::new();
        for part in key.split('\\') {
            if !path.is_empty() {
                path.push('\\');
            }
            path.push_str(part);
            self.keys
                .entry(path.to_lowercase())
                .or_insert_with(|| (path.clone(), Vec::new()));
        }
    }

    fn writable(&self, key: &str) -> Result<(), RegError> {
        let key = key.to_lowercase();
        let closed = self
            .denied
            .iter()
            .any(|d| key == *d || key.starts_with(&format!("{d}\\")));
        if closed {
            Err(RegError::Denied)
        } else {
            Ok(())
        }
    }

    fn values_mut(&mut self, key: &str) -> Result<&mut Vec<Value>, RegError> {
        self.keys
            .get_mut(&key.to_lowercase())
            .map(|(_, values)| values)
            .ok_or(RegError::NotFound)
    }
}

impl Registry for MemoryRegistry {
    fn subkeys(&self, key: &str) -> Result<Vec<String>, RegError> {
        let memory = self.lock();
        let lower = key.to_lowercase();
        if !memory.keys.contains_key(&lower) {
            return Err(RegError::NotFound);
        }
        let prefix = format!("{lower}\\");
        Ok(memory
            .keys
            .iter()
            .filter(|(path, _)| {
                path.strip_prefix(&prefix)
                    .is_some_and(|rest| !rest.contains('\\'))
            })
            .map(|(_, (name, _))| leaf(name).to_string())
            .collect())
    }

    fn values(&self, key: &str) -> Result<Vec<Value>, RegError> {
        let memory = self.lock();
        memory
            .keys
            .get(&key.to_lowercase())
            .map(|(_, values)| values.clone())
            .ok_or(RegError::NotFound)
    }

    fn set_value(&self, key: &str, value: &Value) -> Result<(), RegError> {
        let mut memory = self.lock();
        memory.writable(key)?;
        let values = memory.values_mut(key)?;
        match values.iter_mut().find(|v| same_name(&v.name, &value.name)) {
            Some(old) => *old = value.clone(),
            None => values.push(value.clone()),
        }
        memory.log.push(format!("set {key}\\{}", value.name));
        Ok(())
    }

    fn delete_value(&self, key: &str, name: &str) -> Result<(), RegError> {
        let mut memory = self.lock();
        memory.writable(key)?;
        let values = memory.values_mut(key)?;
        let at = values
            .iter()
            .position(|v| same_name(&v.name, name))
            .ok_or(RegError::NotFound)?;
        values.remove(at);
        memory.log.push(format!("delete-value {key}\\{name}"));
        Ok(())
    }

    fn create_key(&self, key: &str) -> Result<bool, RegError> {
        let mut memory = self.lock();
        if memory.keys.contains_key(&key.to_lowercase()) {
            return Ok(false);
        }
        memory.writable(key)?;
        memory.make(key);
        memory.log.push(format!("create {key}"));
        Ok(true)
    }

    fn delete_tree(&self, key: &str) -> Result<(), RegError> {
        let mut memory = self.lock();
        let lower = key.to_lowercase();
        if !memory.keys.contains_key(&lower) {
            return Err(RegError::NotFound);
        }
        if parent(key).is_none() {
            return Err(RegError::Denied);
        }
        memory.writable(key)?;
        let prefix = format!("{lower}\\");
        memory
            .keys
            .retain(|path, _| *path != lower && !path.starts_with(&prefix));
        memory.log.push(format!("delete-tree {key}"));
        Ok(())
    }

    fn rename_key(&self, key: &str, name: &str) -> Result<(), RegError> {
        let mut memory = self.lock();
        let lower = key.to_lowercase();
        if !memory.keys.contains_key(&lower) {
            return Err(RegError::NotFound);
        }
        let Some(above) = parent(key) else {
            return Err(RegError::Denied);
        };
        memory.writable(key)?;
        let target = join(above, name);
        let target_lower = target.to_lowercase();
        if target_lower != lower && memory.keys.contains_key(&target_lower) {
            // What Windows answers when the name is taken.
            return Err(RegError::Denied);
        }
        let prefix = format!("{lower}\\");
        let moved: Vec<String> = memory
            .keys
            .keys()
            .filter(|path| **path == lower || path.starts_with(&prefix))
            .cloned()
            .collect();
        for path in moved {
            if let Some((cased, values)) = memory.keys.remove(&path) {
                // The same key under the new name: what was after the old name stays.
                let cased = format!("{target}{}", cased.get(key.len()..).unwrap_or(""));
                memory.keys.insert(cased.to_lowercase(), (cased, values));
            }
        }
        memory.log.push(format!("rename {key} -> {name}"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regfile;

    const APP: &str = r"HKEY_CURRENT_USER\Software\Example";

    fn sample() -> MemoryRegistry {
        MemoryRegistry::new()
            .with(
                APP,
                vec![Value::sz("", "default"), Value::dword("Count", 3)],
            )
            .with(&join(APP, "Deep"), vec![Value::sz("a", "b")])
            .with(&join(APP, r"Deep\Deeper"), Vec::new())
            .with(&join(APP, "Other"), Vec::new())
    }

    #[test]
    fn the_system_registry_says_it_is_not_here_where_there_is_none() {
        let reg = SystemRegistry;
        if !SystemRegistry::AVAILABLE {
            assert_eq!(reg.subkeys(ROOTS[0]), Err(RegError::Unsupported));
            assert_eq!(reg.values(ROOTS[0]), Err(RegError::Unsupported));
            assert_eq!(reg.create_key(APP), Err(RegError::Unsupported));
            assert_eq!(reg.delete_tree(APP), Err(RegError::Unsupported));
        }
    }

    #[test]
    fn keys_are_listed_under_the_key_that_holds_them_whatever_the_case() {
        let reg = sample();
        assert_eq!(reg.subkeys(APP).unwrap(), ["Deep", "Other"]);
        assert_eq!(reg.subkeys(&APP.to_uppercase()).unwrap(), ["Deep", "Other"]);
        assert_eq!(reg.subkeys("HKEY_CURRENT_USER").unwrap(), ["Software"]);
        assert_eq!(
            reg.subkeys(r"HKEY_CURRENT_USER\Nope"),
            Err(RegError::NotFound)
        );
        assert_eq!(count_subkeys(&reg, APP), Ok(3));
        assert_eq!(parent(APP), Some(r"HKEY_CURRENT_USER\Software"));
        assert_eq!(parent("HKEY_USERS"), None);
        assert_eq!(leaf(APP), "Example");
    }

    #[test]
    fn a_subtree_goes_out_to_a_file_and_comes_back_into_an_empty_registry() {
        let reg = sample();
        let blocks = export(&reg, APP, true).unwrap();
        let paths: Vec<&str> = blocks.iter().map(|b| b.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                APP.to_string(),
                join(APP, "Deep"),
                join(APP, r"Deep\Deeper"),
                join(APP, "Other")
            ]
        );
        // Not deep: the key and its values alone.
        assert_eq!(export(&reg, APP, false).unwrap().len(), 1);

        let bytes = regfile::encode(&regfile::write(&blocks));
        let read = regfile::parse(&regfile::decode(&bytes)).unwrap();
        let fresh = MemoryRegistry::new();
        apply(&fresh, &read).unwrap();
        assert_eq!(export(&fresh, APP, true).unwrap(), blocks);
    }

    #[test]
    fn a_file_can_take_values_and_keys_away() {
        let reg = sample();
        let text = format!(
            "{}\r\n\r\n[-{APP}\\Deep]\r\n\r\n[{APP}]\r\n\"Count\"=-\r\n\"Gone already\"=-\r\n\
             \r\n[-{APP}\\Never there]\r\n",
            regfile::HEADER
        );
        apply(&reg, &regfile::parse(&text).unwrap()).unwrap();
        assert!(!reg.has_key(&join(APP, "Deep")));
        assert!(!reg.has_key(&join(APP, r"Deep\Deeper")));
        assert!(reg.has_key(&join(APP, "Other")));
        assert_eq!(reg.value(APP, "Count"), None);
        assert_eq!(reg.value(APP, ""), Some(Value::sz("", "default")));
    }

    #[test]
    fn a_value_is_renamed_by_writing_the_new_name_before_removing_the_old() {
        let reg = sample();
        rename_value(&reg, APP, "count", "Total").unwrap();
        assert_eq!(reg.value(APP, "Total"), Some(Value::dword("Total", 3)));
        assert_eq!(reg.value(APP, "Count"), None);
        assert_eq!(
            reg.log(),
            [
                format!("set {APP}\\Total"),
                format!("delete-value {APP}\\count")
            ]
        );
        assert_eq!(
            rename_value(&reg, APP, "Missing", "x"),
            Err(RegError::NotFound)
        );
    }

    #[test]
    fn a_key_is_renamed_with_everything_under_it() {
        let reg = sample();
        reg.rename_key(&join(APP, "Deep"), "Shallow").unwrap();
        assert_eq!(reg.subkeys(APP).unwrap(), ["Other", "Shallow"]);
        assert!(reg.has_key(&join(APP, r"Shallow\Deeper")));
        assert_eq!(
            reg.value(&join(APP, "Shallow"), "a"),
            Some(Value::sz("a", "b"))
        );
        // Onto a name that is taken, and a root: both refused.
        assert_eq!(
            reg.rename_key(&join(APP, "Shallow"), "other"),
            Err(RegError::Denied)
        );
        assert_eq!(reg.rename_key("HKEY_USERS", "x"), Err(RegError::Denied));
    }

    #[test]
    fn a_key_closed_to_writing_refuses_every_change_and_still_reads() {
        let reg = sample();
        reg.deny(APP);
        assert_eq!(
            reg.set_value(APP, &Value::dword("n", 1)),
            Err(RegError::Denied)
        );
        assert_eq!(reg.delete_value(APP, "Count"), Err(RegError::Denied));
        assert_eq!(reg.create_key(&join(APP, "New")), Err(RegError::Denied));
        assert_eq!(reg.delete_tree(&join(APP, "Deep")), Err(RegError::Denied));
        assert_eq!(
            reg.rename_key(&join(APP, "Deep"), "x"),
            Err(RegError::Denied)
        );
        assert_eq!(reg.values(APP).unwrap().len(), 2);
        assert!(reg.log().is_empty());
        // And a file written into it stops at the refusal.
        let blocks = vec![KeyBlock {
            path: APP.into(),
            delete: false,
            lines: vec![Line::Set(Value::dword("n", 1))],
        }];
        assert_eq!(apply(&reg, &blocks), Err(RegError::Denied));
    }

    #[test]
    fn making_a_key_makes_the_keys_above_it_and_says_when_it_was_there() {
        let reg = MemoryRegistry::new();
        assert_eq!(reg.create_key(r"HKEY_CURRENT_USER\A\B\C"), Ok(true));
        assert_eq!(reg.create_key(r"HKEY_CURRENT_USER\a\b"), Ok(false));
        assert_eq!(reg.subkeys(r"HKEY_CURRENT_USER\A").unwrap(), ["B"]);
        assert_eq!(reg.delete_tree("HKEY_CURRENT_USER"), Err(RegError::Denied));
    }
}
