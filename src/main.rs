use anyhow::Result;
use futures_util::{StreamExt, SinkExt, join};
use std::sync::{Arc, atomic::AtomicUsize};

use clap::Parser;

#[derive(Debug, Parser)]
struct Config {
    #[clap(short, long, default_value = "42069")]
    port: usize,

    #[clap(long, default_value = "0.0.0.0")]
    host: String,

    #[clap(short = 'q', long, default_value_t = 1)]
    parallel: usize,

    #[clap(short, long, default_value_t = 100)]
    count: usize,

    #[clap(short, long, default_value_t = 10)]
    rooms_to_join: usize,

    #[clap(short = 'x', long, default_value_t = 20)]
    room_count: usize,

    #[clap(short, long, default_value_t = 10)]
    time_between_messages: u64,

    #[clap(short, long, default_value_t = 100)]
    messages_to_send: usize,
}

async fn run_client(
    url: &'static url::Url,
    rooms: &'static Vec<String>,
    idx: usize,
    config: &'static Config,
) -> Result<usize> {

    let (stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut write, mut read) = stream.split();
    let msg_count = Arc::new(AtomicUsize::new(0));

    let inner_msg_count = msg_count.clone();
    let reader = tokio::spawn(async move {
        let msg_count = inner_msg_count;
        while let Some(_) = read.next().await {
            msg_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    });

    for i in 0..config.rooms_to_join {
        let duration = tokio::time::Duration::from_millis(config.time_between_messages);
        tokio::time::sleep(duration).await;

        let idx = (idx + i) % rooms.len();
        let room = &rooms[idx];

        let msg = format!("JOIN {}", room);
        write.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await?;
    }

    for i in 0..config.messages_to_send {
        let duration = tokio::time::Duration::from_millis(config.time_between_messages);
        tokio::time::sleep(duration).await;

        let idx = (idx + i) % rooms.len();

        let msg = format!("MSG {} hello from idx: {}", idx);
        write.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await?;
    }

    write.close().await?;
    reader.abort();

    return Ok(msg_count.load(std::sync::atomic::Ordering::Relaxed));
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: &'static Config = Box::leak(Box::new(Config::parse()));
    println!("config: {:?}", config);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(config.parallel));
    let url: &'static url::Url = Box::leak(Box::new(url::Url::parse(&format!(
        "ws://{}:{}",
        config.host, config.port
    ))?));
    let rooms = (0..config.room_count)
        .map(|x| format!("room-{}", x))
        .collect::<Vec<String>>();
    let rooms: &'static Vec<String> = Box::leak(Box::new(rooms));
    let message_count = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];
    for i in 0..config.count {
        let permit = semaphore.clone().acquire_owned();
        let message_count = message_count.clone();

        handles.push(tokio::spawn(async move {
            match run_client(url, rooms, i, config).await {
                Ok(count) => {
                    message_count.fetch_add(count, std::sync::atomic::Ordering::Relaxed);
                }
                Err(e) => {
                    eprintln!("Client {} failed: {:?}", i, e);
                }
            };

            drop(permit);
        }));
    }

    futures_util::future::join_all(handles).await;
    println!("messages received: {}", message_count.load(std::sync::atomic::Ordering::Relaxed));

    return Ok(());
}
