use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::os::raw::{c_char, c_void};
use std::path::PathBuf;
use std::time::Duration;
use std::{hint, thread};

use crate::errors::Error;
use crate::{
    create_session, initialize, setup, start_maintenance, Commit, Context, KeyEvent, Session,
    Status, Traits,
};
use cstr::cstr;
use librime_sys::{RimeSessionId, RimeSetNotificationHandler};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DeployResult {
    Success,
    Failure,
}

pub struct Engine {
    pub session: Option<Session>,
    deploy_result: Box<Option<DeployResult>>,
}

pub struct Config {
    pub shared_data_dir: PathBuf,
    pub user_data_dir: PathBuf,
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

    fn get_session(&self) -> Result<&Session, Error> {
        let Some(session) = self.session.as_ref() else {
            return Err(Error::SessionNotExists)
        };
        Ok(session)
    }

    pub fn process_key(&mut self, event: KeyEvent) -> Result<bool, Error> {
        Ok(self.get_session()?.process_key(event))
    }

    pub fn context(&mut self) -> Option<Context> {
        self.session.as_ref()?.context()
    }

    pub fn commit(&mut self) -> Option<Commit> {
        self.session.as_ref()?.commit()
    }

    pub fn status(&mut self) -> Result<Status, Error> {
        self.get_session()?.status().map_err(|_| Error::GetStatus)
    }

    /// Note when using this, function `start_maintenance` needs `full_check` to be `true`.
    pub fn wait_for_deploy_result(&mut self, interval: Duration) -> DeployResult {
        while (*self.deploy_result).is_none() {
            thread::sleep(interval);
            hint::spin_loop();
        }
        (*self.deploy_result).unwrap()
    }

    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(session) = self.session.as_ref() {
            if session.find_session() {
                session.close().map_err(|_| Error::CloseSession)?;
            }
        }
        Ok(())
    }

    pub fn simulate_key_sequence(&self, sequence: &str) -> Result<(), Error> {
        self.get_session()?
            .simulate_key_sequence(sequence)
            .map_err(|_| Error::SimulateKeySequence)?;
        Ok(())
    }

    pub fn create_session(&mut self) -> Result<(), Error> {
        let session = create_session();
        if !session.find_session() {
            return Err(Error::CreateSession);
        }
        self.session = Some(session);
        Ok(())
    }

    pub fn select_schema(&mut self, id: &str) -> Result<(), Error> {
        self.get_session()?.select_schema(id);
        Ok(())
    }
}
