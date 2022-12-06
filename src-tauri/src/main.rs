#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod protocol;

use crate::protocol::{Chat2Message, TOY_CHAT_CONTENT_TOPIC};
// extern crate alloc;
// #[cfg(feature = "alloc")]
// use alloc::vec::Vec;
use chrono::Utc;
use prost::Message;
use serde::ser::{Serialize, SerializeMap, SerializeSeq, SerializeStruct, Serializer};
use std::str::FromStr;
// use std::sync::{Arc, RwLock};
use std::time::Duration;
use tauri::{Manager, RunEvent};
use waku_bindings::{
    waku_new, ContentFilter, Multiaddr, PagingOptions, ProtocolId, Running, StoreQuery,
    WakuNodeHandle,
};

struct Database;

struct Node(WakuNodeHandle<Running>);

#[derive(Debug)]
struct PrettyMessage {
    timestamp: String,
    nick: String,
    message: String,
}

#[derive(serde::Serialize)]
struct CustomResponse {
    message: String,
    other_val: usize,
}

// #[derive(serde::Serialize)]
// struct CustomMessagesResponse {
//     messages: Vec<Chat2Message>,
// }

// // This is what #[derive(Serialize)] would generate.
// impl Serialize for CustomMessagesResponse {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut s = serializer.serialize_struct("CustomMessagesResponse", 1)?;
//         s.serialize_field("messages", &self.messages)?;
//         s.end()
//     }
// }

// #[derive(serde::Serialize)]
// struct VecWrapper(Vec<Chat2Message>);

// // #[cfg(any(feature = "std", feature = "alloc"))]
// impl<T> Serialize for Vec<T>
// where
//     T: Serialize,
// {
//     // impl Serialize for Vec<Chat2Message> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut seq = serializer.serialize_seq(Some(self.len()))?;
//         for e in self {
//             seq.serialize_element(e)?;
//         }
//         seq.end()
//     }
// }

const NODES: &[&str] = &[
    // "/dns4/node-01.ac-cn-hongkong-c.wakuv2.test.statusim.net/tcp/30303/p2p/16Uiu2HAkvWiyFsgRhuJEb9JfjYxEkoHLgnUQmr1N5mKWnYjxYRVm",
    "/dns4/node-01.do-ams3.wakuv2.test.statusim.net/tcp/30303/p2p/16Uiu2HAmPLe7Mzm8TsYUubgCAW1aJoeFScxrLj8ppHFivPo97bUZ",
    // "/dns4/node-01.gc-us-central1-a.wakuv2.test.statusim.net/tcp/30303/p2p/16Uiu2HAmJb2e28qLXxT5kZxVUUoJt72EMzNGXB47Rxx5hw3q4YjS"
];

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

async fn some_other_function() -> Option<String> {
    Some("response".into())
}

#[tauri::command]
async fn my_custom_command(
    window: tauri::Window,
    number: usize,
    database: tauri::State<'_, Database>,
) -> Result<CustomResponse, String> {
    println!("Called from {}", window.label());
    let result: Option<String> = some_other_function().await;
    if let Some(message) = result {
        Ok(CustomResponse {
            message,
            other_val: 42 + number,
        })
    } else {
        Err("No result".into())
    }
}

#[tauri::command]
fn fetch_messages(
    // node_handle: &WakuNodeHandle<Running>,
    node_handle: tauri::State<Node>,
) -> Vec<Chat2Message> {
    // ) -> VecWrapper {
    let self_id = node_handle.0.peer_id().unwrap();
    let peer = node_handle
        .0
        .peers()
        .unwrap()
        .iter()
        .cloned()
        .find(|peer| peer.peer_id() != &self_id)
        .unwrap();

    let result = node_handle
        .0
        .store_query(
            &StoreQuery {
                pubsub_topic: None,
                content_filters: vec![ContentFilter::new(TOY_CHAT_CONTENT_TOPIC.clone())],
                start_time: Some(
                    (Duration::from_secs(Utc::now().timestamp() as u64)
                        - Duration::from_secs(60 * 60 * 24))
                    .as_nanos() as usize,
                ),
                end_time: None,
                paging_options: Some(PagingOptions {
                    page_size: 25,
                    cursor: None,
                    forward: true,
                }),
            },
            peer.peer_id(),
            Some(Duration::from_secs(10)),
        )
        .unwrap();

    let messages: Vec<Chat2Message> = result
        // let messages: Vec<String> = result
        .messages()
        .iter()
        .map(|waku_message| {
            <Chat2Message as Message>::decode(waku_message.payload())
                .expect("Toy chat messages should be decodeable")
            // .message()
        })
        .collect();

    // let messages: Vec<Chat2Message> = Vec::new();
    // if !history.is_empty() {
    //     *messages.write().unwrap() = history;
    // }

    // let result = messages
    //     .read()
    //     .unwrap()
    //     .iter()
    //     .map(|message| PrettyMessage {
    //         timestamp: message
    //             .timestamp()
    //             .unwrap()
    //             .format("%d-%m-%y %H:%M")
    //             .to_string(),
    //         nick: message.nick().to_string(),
    //         message: message.message(),
    //     })
    //     .collect::<Vec<PrettyMessage>>();

    // println!("{:?}", result);

    // VecWrapper(messages)
    messages
}

// fn make_tester(answer: String) -> impl Fn(&str) -> bool {
//     move |challenge| {
//         challenge == answer
//     }
// }

// fn make_fetch_messages(
//     node_handle: &WakuNodeHandle<Running>,
//     // ) -> impl Fn(&WakuNodeHandle<Running>) -> waku_bindings::Result<Vec<Chat2Message>> {
// ) -> impl Fn(&WakuNodeHandle<Running>) -> waku_bindings::Result<Arc<std::sync::RwLock<Vec<Chat2Message>>>>
// {
//     return fetch_messages;
// }

fn main() {
    // Start Waku
    let node_handle = waku_new(None).unwrap();
    let node_handle = node_handle.start().unwrap();

    for address in NODES.iter().map(|a| Multiaddr::from_str(a).unwrap()) {
        let peerid = node_handle.add_peer(&address, ProtocolId::Relay).unwrap();
        node_handle.connect_peer_with_id(peerid, None);
    }

    // #[tauri::command]
    // let foo = make_fetch_messages(&node_handle);

    // Build Tauri
    let app = tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
                window.close_devtools();
            }

            Ok(())
        })
        .manage(Database {})
        .manage(Node(node_handle))
        .invoke_handler(tauri::generate_handler![
            greet,
            my_custom_command,
            fetch_messages
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // todo!: node_handle.stop();
    // Stop Waku
    // https://github.com/spacedriveapp/spacedrive/blob/4dc7c4571afee0ea17567115e3168b2dcbd7c508/apps/desktop/src-tauri/src/main.rs for example
    // app.run(move |app_handler, event| {
    //     if let RunEvent::ExitRequested { .. } = event {
    //         // println!("Closing all open windows...");

    //         // block_in_place(|| block_on(node.shutdown()));

    //         // app_handler.exit(0);
    //     }
    // });
}
