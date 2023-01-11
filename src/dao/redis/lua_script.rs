use lazy_static::lazy_static;

lazy_static! {
    pub static ref ROOMS_CHANGE: String = load_rooms_change();
}

pub fn load_rooms_change() -> String {
    let mut conn = utilities::redis::sync::conn_sync().unwrap();
    let files = ["json.lua", "rooms.lua"];
    let cmd: String = files
        .iter()
        .map(|&name| std::fs::read_to_string(format!("static/lua_scripts/{}", name)).unwrap())
        .collect();

    // utilities::redis::redis::cmd();
    redis::cmd("script")
        .arg("load")
        .arg(cmd)
        .query::<String>(&mut conn)
        .unwrap() // return sha
}
