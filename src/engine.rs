use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::time::Duration;
use std::{hint, thread};

use cstr::cstr;
use librime_sys::{RimeSessionId, RimeSetNotificationHandler};

use crate::errors::Error;
use crate::{create_session, initialize, setup, start_maintenance, Session, Traits};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DeployResult {
    Success,
    Failure,
}

pub struct Engine {
    session: Option<Session>,
    deploy_result: Box<Option<DeployResult>>,
}

impl Engine {
    pub fn new(mut traits: Traits) -> Engine {
        setup(&mut traits);

        extern "C" fn notification_handler(
            obj: *mut c_void,
            _session_id: RimeSessionId,
            message_type: *const c_char,
            message_value: *const c_char,
        ) {
            unsafe {
                let deploy_result = &mut *(obj as *mut Option<DeployResult>);
                let message_type = CStr::from_ptr(message_type);
                let message_value = CStr::from_ptr(message_value);
                if message_type == cstr!("deploy") {
                    match message_value {
                        _ if message_value == cstr!("success") => {
                            (*deploy_result).replace(DeployResult::Success);
                        }
                        _ if message_value == cstr!("failure") => {
                            (*deploy_result).replace(DeployResult::Failure);
                        }
                        _ => {}
                    }
                }
            }
        }

        let mut deploy_result = Box::new(None);

        unsafe {
            RimeSetNotificationHandler(
                Some(notification_handler),
                &mut *deploy_result as *mut Option<DeployResult> as *mut c_void,
            );
        }

        initialize(&mut traits);
        start_maintenance(true);
        Self {
            session: None,
            deploy_result,
        }
    }

    /// Note when using this, function `start_maintenance` needs `full_check` to be `true`.
    pub fn wait_for_deploy_result(&mut self, interval: Duration) -> DeployResult {
        while (*self.deploy_result).is_none() {
            thread::sleep(interval);
            hint::spin_loop();
        }
        (*self.deploy_result).unwrap()
    }

    pub fn create_session(&mut self) -> Result<(), Error> {
        self.silently_close_session();
        let session = create_session()?;
        self.session = Some(session);
        Ok(())
    }

    fn silently_close_session(&self) {
        if let Some(session) = self.session.as_ref() {
            if session.find_session() {
                let _ = session.close();
            }
        }
    }

    /// Returns `None` if the session hasn't been created.
    ///
    /// Call `create_session()` to create a session.
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.silently_close_session();
    }
}
