use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::sync::Mutex;
use std::time::Duration;
use std::{hint, thread};

use librime_sys::{RimeSessionId, RimeSetNotificationHandler};
use once_cell::sync::Lazy;

use crate::errors::Error;
use crate::{create_session, initialize, setup, start_maintenance, Session, Traits};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DeployResult {
    Success,
    Failure,
}

pub struct Engine {
    session: Option<Session>,
    rime_message_handler_data: Box<RimeMessageHandlerData>,
}

static SETUP_INIT_FLAG: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

impl Engine {
    pub fn new(mut traits: Traits) -> Engine {
        let setup_init_flag = *SETUP_INIT_FLAG.lock().unwrap();
        if !setup_init_flag {
            setup(&mut traits);
            *SETUP_INIT_FLAG.lock().unwrap() = true;
        }

        extern "C" fn notification_handler(
            obj: *mut c_void,
            _session_id: RimeSessionId,
            message_type: *const c_char,
            message_value: *const c_char,
        ) {
            unsafe {
                let data = &mut *(obj as *mut RimeMessageHandlerData);
                let deploy_result = &mut data.deploy_result;
                let message_type = CStr::from_ptr(message_type).to_str().unwrap();
                let message_value = CStr::from_ptr(message_value).to_str().unwrap();
                if message_type == "deploy" {
                    match message_value {
                        _ if message_value == "success" => {
                            (*deploy_result).replace(DeployResult::Success);
                        }
                        _ if message_value == "failure" => {
                            (*deploy_result).replace(DeployResult::Failure);
                        }
                        _ => {}
                    }
                }

                if let Some(f) = &data.user_handler {
                    (**f)(message_type, message_value);
                }
            }
        }

        let mut rime_message_handler_data = Box::new(RimeMessageHandlerData {
            deploy_result: None,
            user_handler: None,
        });

        unsafe {
            RimeSetNotificationHandler(
                Some(notification_handler),
                &mut *rime_message_handler_data as *mut RimeMessageHandlerData as *mut c_void,
            );
        }

        initialize(&mut traits);
        start_maintenance(true);
        Self {
            session: None,
            rime_message_handler_data,
        }
    }

    pub fn set_notification_callback<F>(&mut self, callback: F)
    where
        F: Fn(&str, &str) + 'static,
    {
        self.rime_message_handler_data
            .user_handler
            .replace(Box::new(callback));
    }

    /// Note when using this, function `start_maintenance` needs `full_check` to be `true`.
    pub fn wait_for_deploy_result(&mut self, interval: Duration) -> DeployResult {
        let deploy_result = &self.rime_message_handler_data.deploy_result;
        while deploy_result.is_none() {
            thread::sleep(interval);
            hint::spin_loop();
        }
        deploy_result.unwrap()
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

type NotificationHandlerFn = dyn Fn(&str, &str) + 'static;

struct RimeMessageHandlerData {
    deploy_result: Option<DeployResult>,
    user_handler: Option<Box<NotificationHandlerFn>>,
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.silently_close_session();
    }
}
