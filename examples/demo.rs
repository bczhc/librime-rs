use std::io::stdin;

use librime_sys::RimeKeyCode_XK_f;

use rime_api::{
    create_session, finalize, initialize, KeyEvent, setup, start_maintenance, Traits,
};

fn main() {
    let mut traits = Traits::new();
    traits.set_shared_data_dir("/usr/share/rime-data");
    traits.set_user_data_dir("/home/bczhc/.local/share/fcitx5/rime");
    traits.set_distribution_name("Rime");
    traits.set_distribution_code_name("Rime");
    traits.set_distribution_version("0.0.0");
    traits.set_app_name("rime-demo");
    setup(&mut traits);
    initialize(&mut traits);
    start_maintenance(false);
    let mut session = create_session();
    session.select_schema("tiger");

    let stdin = stdin();
    loop {
        stdin.read_line(&mut String::new()).unwrap();
        if !session.find_session() {
            session = create_session();
        }
        let event = KeyEvent::new(RimeKeyCode_XK_f, 0);
        let result = session.process_key(event);
        if result.is_err() {
            println!("ProcessKey: Error");
        }
        println!("{:?}", session.context());
        println!("{:?}", session.commit());
    }

    let _ = session.close();
    finalize();
}
