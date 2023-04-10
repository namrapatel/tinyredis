use assert_cmd::prelude::CommandCargoExt;
use redis::Client;
use std::{
    process::{Command, Stdio},
    thread::sleep,
    time::Duration,
};

static mut PROC: Option<std::process::Child> = None;

#[ctor::ctor]
fn start_server() {
    let mut cmd = Command::cargo_bin("tinyredis").unwrap();
    unsafe {
        PROC = Some(cmd.stdout(Stdio::null()).spawn().unwrap());
    }
    sleep(Duration::from_millis(100));
}

#[ctor::dtor]
fn stop_server() {
    unsafe {
        if let Some(ref mut proc) = PROC {
            proc.kill().unwrap();
        }
    }
}

#[test]
fn it_can_connect_to_redis() {
    let client = Client::open("redis://127.0.0.1/").unwrap();
    let con = client.get_connection();

    assert!(con.is_ok());
}

#[test]
fn it_can_handle_multiple_connections() {
    let client = Client::open("redis://127.0.0.1/").unwrap();
    let con = client.get_connection();
    let con2 = client.get_connection();

    assert!(con.is_ok());
    assert!(con2.is_ok());
}

#[test]
fn it_can_receive_pings() {
    let client = Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();

    let ping: String = redis::cmd("PING").query(&mut con).unwrap();
    assert_eq!(ping, "PONG");

    // let ping: String = redis::cmd("PING").arg("hello").query(&mut con).unwrap();
    // assert_eq!(ping, "hello");
}

#[test]
fn it_can_handle_pings_from_multiple_connections() {
    let client = Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();
    let mut con2 = client.get_connection().unwrap();

    let ping: String = redis::cmd("PING").query(&mut con).unwrap();
    let ping2: String = redis::cmd("PING").query(&mut con).unwrap();
    assert_eq!(ping, "PONG");
    assert_eq!(ping2, "PONG");
}

#[test]
fn it_can_handle_echo() {
    let client = Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();

    let echo: String = redis::cmd("ECHO").arg("hello").query(&mut con).unwrap();
    assert_eq!(echo, "hello");
}

#[test]
// fn it_can_get_and_set() {
//     let client = Client::open("redis://127.0.0.1/").unwrap();
//     let mut con = client.get_connection().unwrap();
//     let mut con2 = client.get_connection().unwrap();

//     let err = redis::cmd("GET")
//         .arg("key")
//         .query::<String>(&mut con)
//         .unwrap_err();

//     println!("{:?}", err);

//     assert_eq!(
//         err.detail().unwrap(),
//         "\"Response type not string compatible.\" (response was nil)"
//     );

//     let _ = redis::cmd("SET")
//         .arg("key")
//         .arg("value")
//         .query::<String>(&mut con)
//         .unwrap();

//     let value: String = redis::cmd("GET").arg("key").query(&mut con).unwrap();
//     assert_eq!(value, "value");

//     let value: String = redis::cmd("GET").arg("key").query(&mut con2).unwrap();
//     assert_eq!(value, "value");
// }
#[test]
fn it_can_set_with_ttl() {
    let client = Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();

    let _ = redis::cmd("SET")
        .arg("key-ttl")
        .arg("value")
        .arg("PX")
        .arg(1000)
        .query::<String>(&mut con)
        .unwrap();

    let value: String = redis::cmd("GET").arg("key-ttl").query(&mut con).unwrap();
    assert_eq!(value, "value");

    sleep(Duration::from_millis(2000));

    let err = redis::cmd("GET")
        .arg("key-ttl")
        .query::<String>(&mut con)
        .unwrap_err();
    assert_eq!(
        err.detail().unwrap(),
        "\"Response type not string compatible.\" (response was nil)"
    );
}

#[test]
fn it_can_handle_del() {
    let client = Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();

    let _ = redis::cmd("SET")
        .arg("del2")
        .arg("del2")
        .query::<String>(&mut con)
        .unwrap();

    let value: String = redis::cmd("GET").arg("del2").query(&mut con).unwrap();
    assert_eq!(value, "del2");

    let del: String = redis::cmd("DEL").arg("del2 del3").query(&mut con).unwrap();
    assert_eq!(del, "1");
}

#[test]
fn it_can_handle_LRU() {
    let client = Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();

    let _ = redis::cmd("SET")
        .arg("over1")
        .arg("value1")
        .query::<String>(&mut con)
        .unwrap();

    let _ = redis::cmd("SET")
        .arg("over2")
        .arg("value2")
        .query::<String>(&mut con)
        .unwrap();

    let _ = redis::cmd("SET")
        .arg("over3")
        .arg("value3")
        .query::<String>(&mut con)
        .unwrap();

    let _ = redis::cmd("SET")
        .arg("over4")
        .arg("value4")
        .query::<String>(&mut con)
        .unwrap();

    // over4 have smallest aging, over1 have largest againg, LRU
    let value: String = redis::cmd("GET").arg("over4").query(&mut con).unwrap();
    assert_eq!(value, "value4");

    // LRU Policy remove over1
    let err = redis::cmd("GET")
        .arg("over1")
        .query::<String>(&mut con)
        .unwrap_err();
    assert_eq!(
        err.detail().unwrap(),
        "\"Response type not string compatible.\" (response was nil)"
    );
}
