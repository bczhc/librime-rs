use std::io::{stdin, BufRead};

use rime_api::{
    create_session, full_deploy_and_wait, initialize, set_notification_handler, setup,
    DeployResult, Traits,
};

fn main() {
    let mut traits = Traits::new();
    traits.set_shared_data_dir("/usr/share/rime-data");
    traits.set_user_data_dir("/home/bczhc/.local/share/fcitx5/rime");
    traits.set_distribution_name("Rime");
    traits.set_distribution_code_name("Rime");
    traits.set_distribution_version("0.0.0");
    traits.set_app_name("rime-demo");

    println!("---------- Traits: ----------");
    println!("{:?}", traits);
    println!("-----------------------------");

    setup(&mut traits);
    initialize(&mut traits);

    set_notification_handler(|t, v| {
        println!("Notification message: {:?}", (t, v));
    });

    let deploy_result = full_deploy_and_wait();
    match deploy_result {
        DeployResult::Success => {
            println!("Deployment done");
        }
        DeployResult::Failure => {
            panic!("Deployment failed");
        }
    }

    let mut session = create_session().unwrap();
    session.select_schema("092wubi").unwrap();

    let mut stdin = stdin().lock();
    loop {
        let mut line = String::new();
        stdin.read_line(&mut line).unwrap();
        let line = line.strip_suffix('\n').unwrap();

        if line == "exit" {
            session.close().unwrap();
            break;
        }

        // let event = KeyEvent::new(RimeKeyCode_XK_g, 0);
        // println!("{:?}", session.process_key(event));
        session.simulate_key_sequence(line).unwrap();
        let c = session.context();
        println!("{:?}", c);
        if let Some(c) = c {
            println!("{:?}", c.composition());
            println!("{:?}", c.menu());
        }
        println!("{:?}", session.commit());
    }
}
