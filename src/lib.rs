use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

use librime_sys::{
    rime_struct, RimeCommit, RimeContext, RimeCreateSession, RimeDestroySession, RimeFinalize,
    RimeFindSession, RimeFreeCommit, RimeFreeContext, RimeFreeStatus, RimeGetCommit,
    RimeGetContext, RimeGetStatus, RimeInitialize, RimeKeyCode, RimeModifier, RimeProcessKey,
    RimeSelectSchema, RimeSessionId, RimeSetup, RimeSimulateKeySequence, RimeStartMaintenance,
    RimeStatus,
};

use crate::errors::{Error, Result};

pub mod engine;
pub mod errors;

macro_rules! new_c_string {
    ($x:expr) => {
        std::ffi::CString::new($x).expect("CString creation failed")
    };
}

pub struct Traits {
    inner: librime_sys::RimeTraits,
    resources: Vec<*mut c_char>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum KeyStatus {
    Accept,
    Pass,
}

macro_rules! setter_fn_impl {
    ($field_name:ident, $fn_name:ident) => {
        impl Traits {
            // TODO: support `Path`
            pub fn $fn_name(&mut self, path: &str) -> &mut Self {
                let c_string = CString::new(path).expect("CString creation failed");
                let ptr = c_string.into_raw();
                self.inner.$field_name = ptr;
                self.resources.push(ptr);
                self
            }
        }
    };
}

setter_fn_impl!(shared_data_dir, set_shared_data_dir);
setter_fn_impl!(user_data_dir, set_user_data_dir);
setter_fn_impl!(distribution_name, set_distribution_name);
setter_fn_impl!(distribution_code_name, set_distribution_code_name);
setter_fn_impl!(distribution_version, set_distribution_version);
setter_fn_impl!(app_name, set_app_name);
setter_fn_impl!(log_dir, set_log_dir);
setter_fn_impl!(prebuilt_data_dir, set_prebuilt_data_dir);
setter_fn_impl!(staging_dir, set_staging_dir);

impl Traits {
    pub fn new() -> Self {
        rime_struct!(rime_traits: librime_sys::RimeTraits);
        Self {
            inner: rime_traits,
            resources: Vec::new(),
        }
    }

    pub fn set_min_log_level(&mut self, level: u8) -> &mut Self {
        self.inner.min_log_level = level as c_int;
        self
    }

    pub fn set_modules(&mut self, _modules: &[&str]) -> &mut Self {
        todo!()
    }
}

impl Default for Traits {
    fn default() -> Self {
        Self::new()
    }
}

pub fn setup(traits: &mut Traits) {
    unsafe {
        RimeSetup(&mut traits.inner);
    }
}

pub fn initialize(traits: &mut Traits) {
    unsafe {
        RimeInitialize(&mut traits.inner);
    }
}

pub fn finalize() {
    unsafe {
        RimeFinalize();
    }
}

impl Drop for Traits {
    fn drop(&mut self) {
        for x in &self.resources {
            unsafe {
                drop(CString::from_raw(*x));
            }
        }
    }
}

pub fn start_maintenance(full_check: bool) -> bool {
    unsafe { RimeStartMaintenance(full_check as c_int) != 0 }
}

pub fn create_session() -> Result<Session> {
    let session_id = unsafe { RimeCreateSession() };
    let session = Session { session_id };
    if !session.find_session() {
        return Err(Error::CreateSession);
    }
    Ok(session)
}

pub struct Session {
    session_id: RimeSessionId,
}

impl Session {
    pub fn find_session(&self) -> bool {
        unsafe { RimeFindSession(self.session_id) != 0 }
    }

    #[allow(clippy::result_unit_err)]
    pub fn select_schema(&self, id: &str) -> bool {
        unsafe {
            let s = new_c_string!(id);
            RimeSelectSchema(self.session_id, s.as_ptr()) != 0
        }
    }

    pub fn process_key(&self, key: KeyEvent) -> KeyStatus {
        let status = unsafe { RimeProcessKey(self.session_id, key.key_code, key.modifiers) };
        if status != 0 {
            KeyStatus::Accept
        } else {
            KeyStatus::Pass
        }
    }

    pub fn context(&self) -> Option<Context> {
        unsafe {
            rime_struct!(context: RimeContext);
            if RimeGetContext(self.session_id, &mut context) == 0 {
                return None;
            }
            Some(Context { inner: context })
        }
    }

    pub fn commit(&self) -> Option<Commit> {
        rime_struct!(commit: RimeCommit);
        unsafe {
            if RimeGetCommit(self.session_id, &mut commit) == 0 {
                return None;
            }
        }
        Some(Commit { inner: commit })
    }

    #[allow(clippy::result_unit_err)]
    pub fn close(&self) -> Result<()> {
        unsafe {
            if RimeDestroySession(self.session_id) == 0 {
                Err(Error::CloseSession)
            } else {
                Ok(())
            }
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn status(&self) -> Result<Status> {
        rime_struct!(status: RimeStatus);
        unsafe {
            if RimeGetStatus(self.session_id, &mut status) == 0 {
                Err(Error::GetStatus)
            } else {
                let r = Status {
                    inner: status,
                    schema_id: to_c_str(status.schema_id),
                    schema_name: to_c_str(status.schema_name),
                    is_disabled: status.is_disabled != 0,
                    is_composing: status.is_composing != 0,
                    is_ascii_mode: status.is_ascii_mode != 0,
                    is_full_shape: status.is_full_shape != 0,
                    is_simplified: status.is_simplified != 0,
                    is_traditional: status.is_traditional != 0,
                    is_ascii_punct: status.is_ascii_punct != 0,
                };
                Ok(r)
            }
        }
    }

    pub fn simulate_key_sequence(&self, key_sequence: &str) -> Result<()> {
        unsafe {
            let key_sequence = CString::new(key_sequence)?;
            if RimeSimulateKeySequence(self.session_id, key_sequence.as_ptr()) == 1 {
                Ok(())
            } else {
                Err(Error::SimulateKeySequence)
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct KeyEvent {
    pub key_code: i32,
    pub modifiers: i32,
}

impl KeyEvent {
    pub fn new(key_code: RimeKeyCode, modifiers: RimeModifier) -> Self {
        Self {
            key_code: key_code as i32,
            modifiers: modifiers as i32,
        }
    }
}

/// Context of a Rime session
///
/// This type doesn't need a lifetime parameter
/// since it stores full text (copies are done in librime)
/// on the heap once `Session::context()` is called,
/// and uses `RimeFreeContext` to free them in `drop()`.
///
/// Same for `Commit`.
#[derive(Debug)]
pub struct Context {
    inner: RimeContext,
}

impl Context {
    pub fn composition(&self) -> Composition<'_> {
        let composition = self.inner.composition;
        Composition {
            length: composition.length as usize,
            cursor_pos: composition.cursor_pos as usize,
            sel_start: composition.sel_start as usize,
            sel_end: composition.sel_end as usize,
            preedit: to_c_str_nullable(composition.preedit),
        }
    }

    pub fn menu(&self) -> Menu<'_> {
        let menu = self.inner.menu;

        Menu {
            page_size: menu.page_size as usize,
            page_no: menu.page_no as usize,
            is_last_page: menu.is_last_page == librime_sys::True as i32,
            highlighted_candidate_index: menu.highlighted_candidate_index as usize,
            num_candidates: menu.num_candidates as usize,
            candidates: unsafe {
                let mut candidates = Vec::new();
                for i in 0..menu.num_candidates as usize {
                    let candidate = &*menu.candidates.add(i);
                    candidates.push(Candidate {
                        text: to_c_str(candidate.text),
                        comment: to_c_str_nullable(candidate.comment),
                    });
                }
                candidates
            },
            select_keys: to_c_str_nullable(menu.select_keys),
        }
    }

    pub fn select_labels(&self) -> Option<Vec<&'_ str>> {
        to_c_str_vec(self.inner.select_labels, self.inner.menu.page_size as usize)
    }

    pub fn raw(&self) -> RimeContext {
        self.inner
    }
}

#[derive(Debug)]
pub struct Composition<'a> {
    pub length: usize,
    pub cursor_pos: usize,
    pub sel_start: usize,
    pub sel_end: usize,
    pub preedit: Option<&'a str>,
}

#[derive(Debug)]
pub struct Menu<'a> {
    pub page_size: usize,
    pub page_no: usize,
    pub is_last_page: bool,
    pub highlighted_candidate_index: usize,
    pub num_candidates: usize,
    pub candidates: Vec<Candidate<'a>>,
    pub select_keys: Option<&'a str>,
}

#[derive(Debug)]
pub struct Candidate<'a> {
    pub text: &'a str,
    pub comment: Option<&'a str>,
}

fn to_c_str<'a>(ptr: *mut c_char) -> &'a str {
    to_c_str_nullable(ptr).unwrap()
}

fn to_c_str_nullable<'a>(ptr: *mut c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    Some(to_c_str(ptr))
}

fn to_c_str_vec<'a>(ptr: *mut *mut c_char, length: usize) -> Option<Vec<&'a str>> {
    if ptr.is_null() {
        return None;
    }
    let mut vec = Vec::with_capacity(length);
    for i in 0..length {
        unsafe {
            vec.push(to_c_str(*ptr.add(i)));
        }
    }
    Some(vec)
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            RimeFreeContext(&mut self.inner);
        }
    }
}

#[derive(Debug)]
pub struct Commit {
    inner: RimeCommit,
}

impl Commit {
    pub fn text(&self) -> &'_ str {
        to_c_str(self.inner.text)
    }
}

impl Drop for Commit {
    fn drop(&mut self) {
        unsafe {
            RimeFreeCommit(&mut self.inner);
        }
    }
}

pub struct Status<'a> {
    inner: RimeStatus,
    pub schema_id: &'a str,
    pub schema_name: &'a str,
    pub is_disabled: bool,
    pub is_composing: bool,
    pub is_ascii_mode: bool,
    pub is_full_shape: bool,
    pub is_simplified: bool,
    pub is_traditional: bool,
    pub is_ascii_punct: bool,
}

impl<'a> Drop for Status<'a> {
    #[allow(clippy::result_unit_err)]
    fn drop(&mut self) {
        unsafe {
            let _ = RimeFreeStatus(&mut self.inner);
        }
    }
}

pub fn default_user_data_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    match home::home_dir() {
        None => PathBuf::new(),
        Some(mut home) => {
            home.push(".local/share/fcitx5/rime");
            home
        }
    }

    #[cfg(not(target_os = "linux"))]
    // TODO
    PathBuf::new()
}

pub fn default_shared_data_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    let dir = PathBuf::from("/usr/share/rime-data/");
    #[cfg(not(target_os = "linux"))]
    // TODO
    let dir = PathBuf::new();

    dir
}
