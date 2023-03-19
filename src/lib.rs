use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

use librime_sys::{
    rime_struct, RimeCommit, RimeContext, RimeCreateSession, RimeDestroySession, RimeFinalize,
    RimeFindSession, RimeFreeCommit, RimeFreeContext, RimeFreeStatus, RimeGetCommit,
    RimeGetContext, RimeGetStatus, RimeInitialize, RimeKeyCode, RimeModifier, RimeProcessKey,
    RimeSelectSchema, RimeSessionId, RimeSetup, RimeSimulateKeySequence, RimeStartMaintenance,
    RimeStatus, RimeTraits,
};

macro_rules! new_c_string {
    ($x:expr) => {
        std::ffi::CString::new($x).expect("CString creation failed")
    };
}

pub struct Traits {
    inner: RimeTraits,
    resources: Vec<*mut c_char>,
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
        rime_struct!(rime_traits: RimeTraits);
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

pub fn create_session() -> Session {
    unsafe {
        let session_id = RimeCreateSession();
        Session { session_id }
    }
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

    pub fn process_key(&self, key: KeyEvent) -> bool {
        let status = unsafe { RimeProcessKey(self.session_id, key.key_code, key.modifiers) };
        status != 0
    }

    pub fn context(&self) -> Option<Context> {
        unsafe {
            rime_struct!(context: RimeContext);
            if RimeGetContext(self.session_id, &mut context) == 0 {
                return None;
            }
            let ret = Context {
                inner: context,
                composition: Composition {
                    length: context.composition.length,
                    cursor_pos: context.composition.cursor_pos,
                    sel_start: context.composition.sel_start,
                    sel_end: context.composition.sel_end,
                    preedit: to_c_str_nullable(context.composition.preedit),
                },
                menu: Menu {
                    page_size: context.menu.page_size,
                    page_no: context.menu.page_no,
                    is_last_page: context.menu.is_last_page != 0,
                    highlighted_candidate_index: context.menu.highlighted_candidate_index,
                    num_candidates: context.menu.num_candidates,
                    candidates: {
                        let mut vec = Vec::new();
                        let num = context.menu.num_candidates;
                        for i in 0..(num as usize) {
                            let c = context.menu.candidates.add(i);
                            let c = Candidate {
                                text: to_c_str((*c).text),
                                comment: to_c_str_nullable((*c).comment),
                            };
                            vec.push(c);
                        }
                        vec
                    },
                    select_keys: vec![],
                    /* TODO */
                },
            };
            Some(ret)
        }
    }

    pub fn commit(&self) -> Option<Commit> {
        rime_struct!(commit: RimeCommit);
        unsafe {
            if RimeGetCommit(self.session_id, &mut commit) == 0 {
                return None;
            }
        }
        Some(Commit {
            inner: commit,
            text: to_c_str(commit.text),
        })
    }

    #[allow(clippy::result_unit_err)]
    pub fn close(&self) -> Result<(), ()> {
        unsafe {
            if RimeDestroySession(self.session_id) == 0 {
                Err(())
            } else {
                Ok(())
            }
        }
    }

    #[allow(clippy::result_unit_err)]
    pub fn status(&self) -> Result<Status, ()> {
        rime_struct!(status: RimeStatus);
        unsafe {
            if RimeGetStatus(self.session_id, &mut status) == 0 {
                Err(())
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

    pub fn simulate_key_sequence(&self, key_sequence: &str) -> Result<(), ()> {
        unsafe {
            let key_sequence = CString::new(key_sequence).map_err(|_| ())?;
            if RimeSimulateKeySequence(self.session_id, key_sequence.as_ptr()) == 1 {
                Ok(())
            } else {
                Err(())
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

#[derive(Debug)]
pub struct Context<'a> {
    inner: RimeContext,
    pub composition: Composition<'a>,
    pub menu: Menu<'a>,
}

#[derive(Debug)]
pub struct Composition<'a> {
    pub length: i32,
    pub cursor_pos: i32,
    pub sel_start: i32,
    pub sel_end: i32,
    pub preedit: Option<&'a str>,
}

#[derive(Debug)]
pub struct Menu<'a> {
    pub page_size: i32,
    pub page_no: i32,
    pub is_last_page: bool,
    pub highlighted_candidate_index: i32,
    pub num_candidates: i32,
    pub candidates: Vec<Candidate<'a>>,
    pub select_keys: Vec<&'a str>,
}

#[derive(Debug)]
pub struct Candidate<'a> {
    pub text: &'a str,
    pub comment: Option<&'a str>,
}

fn to_c_str<'a>(ptr: *mut c_char) -> &'a str {
    unsafe { CStr::from_ptr(ptr).to_str().expect("Invalid UTF-8") }
}

fn to_c_str_nullable<'a>(ptr: *mut c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    Some(to_c_str(ptr))
}

impl<'a> Drop for Context<'a> {
    fn drop(&mut self) {
        unsafe {
            let _ = RimeFreeContext(&mut self.inner);
        }
    }
}

#[derive(Debug)]
pub struct Commit<'a> {
    inner: RimeCommit,
    pub text: &'a str,
}

impl<'a> Drop for Commit<'a> {
    fn drop(&mut self) {
        unsafe {
            let _ = RimeFreeCommit(&mut self.inner);
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
