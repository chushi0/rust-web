use core::time;
use idl_gen::bss_websocket_client::BoxProtobufPayload;
use protobuf::Message;
use std::collections::HashMap;
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};

#[derive(Default)]
pub struct InputManager {
    list: Mutex<HashMap<i64, Input>>,
}

struct Input {
    expect: String,
    sender: Sender<Vec<u8>>,
}

impl InputManager {
    pub async fn wait_for_input<T, F, L>(
        &self,
        user_id: i64,
        timeout: time::Duration,
        timeout_val: F,                   // 等待连接超时时，默认取值
        on_start_listen_input: Option<L>, // 开始监听输入时调用，用于通知客户端“可以发送消息”
    ) -> T
    where
        T: Message,
        F: Fn() -> T,
        L: Fn(),
    {
        let (sender, mut recv) = mpsc::channel(1);

        let input = Input {
            expect: T::NAME.to_string(),
            sender,
        };

        self.list.lock().await.insert(user_id, input);

        if let Some(on_start_listen_input) = on_start_listen_input {
            on_start_listen_input();
        }

        tokio::select! {
            _ = tokio::time::sleep(timeout) => {
                self.list.lock().await.remove(&user_id);

                timeout_val()
            }

            data = recv.recv() => {
                if let Some(data) = data {
                    if let Ok(data) =  T::parse_from_bytes(&data) {
                        data
                    } else {
                        timeout_val()
                    }
                } else {
                    timeout_val()
                }
            }
        }
    }

    pub async fn player_input(&self, user_id: i64, data: BoxProtobufPayload) {
        let mut list = self.list.lock().await;
        let input = list.get(&user_id);
        if let Some(input) = input {
            if input.expect != data.name {
                // ignore other input
                log::info!(
                    "ignore unexpected input: {}, expected: {}",
                    data.name,
                    input.expect
                );
                return;
            }
            let _ = input.sender.send(data.payload).await; // ignore error
            list.remove(&user_id);
        }
    }
}