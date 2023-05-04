use std::io::{stdin, BufRead};
use std::time::Duration;

use librime_sys::RimeKeyCode_XK_g;

use rime_api::engine::{DeployResult, Engine};
use rime_api::{KeyEvent, Traits};

fn main() {
    let mut traits = Traits::new();
    traits.set_shared_data_dir("/usr/share/rime-data");
    traits.set_user_data_dir("/home/bczhc/.local/share/fcitx5/rime");
    traits.set_distribution_name("Rime");
    traits.set_distribution_code_name("Rime");
    traits.set_distribution_version("0.0.0");
    traits.set_app_name("rime-demo");

    let mut engine = Engine::new(traits);
    let deploy_result = engine.wait_for_deploy_result(Duration::from_secs_f32(0.1));
    match deploy_result {
        DeployResult::Success => {
            println!("Deployment done");
        }
        DeployResult::Failure => {
            panic!("Deployment failed");
        }
    }

    engine.create_session().unwrap();
    let session = engine.session().unwrap();
    session.select_schema("092wubi");

    let mut stdin = stdin().lock();
    loop {
        stdin.read_line(&mut String::new()).unwrap();
        let event = KeyEvent::new(RimeKeyCode_XK_g, 0);
        println!("{:?}", session.process_key(event));
        println!("{:?}", session.context());
        println!("{:?}", session.commit());
    }
}
