use anyhow::Result;
use dotenv::dotenv;
use futures_util::{SinkExt, StreamExt};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize},
    Arc,
};

use clap::Parser;

#[derive(Debug, Parser)]
struct Config {
    #[clap(env, short, long, env, default_value = "42069")]
    port: usize,

    #[clap(env, long, env, default_value = "1337")]
    stop_port: usize,

    #[clap(long, default_value = "0.0.0.0")]
    host: String,

    #[clap(env, short = 'q', long, default_value_t = 1)]
    parallel: usize,

    #[clap(env, short, long, default_value_t = 1)]
    count: usize,

    #[clap(env, short, long, default_value_t = "/tmp/rooms.csv")]
    file: PathBuf,

    #[clap(env, short, long, default_value_t = 2)]
    rooms_to_join: usize,

    #[clap(env, short = 'x', long, default_value_t = 20)]
    room_count: usize,

    #[clap(env, short, long, default_value_t = 10)]
    time_between_messages: u64,

    #[clap(env, long, default_value_t = 10)]
    time_between_connections: u64,

    #[clap(env, short, long, default_value_t = 1)]
    messages_to_send: usize,
}

async fn run_client(
    url: &'static url::Url,
    rooms: &'static Vec<String>,
    idx: usize,
    config: &'static Config,
) -> Result<(usize, usize, bool)> {
    let idx = idx + 1;
    let (stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (mut write, mut read) = stream.split();
    let msg_count = Arc::new(AtomicUsize::new(0));
    let timeout = Arc::new(AtomicBool::new(false));
    let now = std::time::Instant::now();

    let inner_msg_count = msg_count.clone();
    let inner_timeout = timeout.clone();
    let reader = tokio::spawn(async move {
        let msg_count = inner_msg_count;
        let timeout = inner_timeout;
        let mut my_message_count = config.messages_to_send;
        let now = std::time::Instant::now();

        loop {
            let time_between = std::cmp::max(1, config.time_between_messages);
            let time_left = config.messages_to_send * time_between as usize * 3;
            let time_left =
                (std::cmp::max(1000, time_left)).saturating_sub(now.elapsed().as_millis() as usize);

            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(time_left as u64)) => {
                    timeout.store(true, std::sync::atomic::Ordering::Relaxed);
                    break;
                }
                Some(Ok(msg)) = read.next() => {
                    msg_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    let (id, _) = msg.to_text().unwrap().split_once(":").unwrap();
                    let id = id.split(" ").last().unwrap();
                    let id = id.parse::<usize>().unwrap();
                    if id == idx {
                        my_message_count = my_message_count.saturating_sub(1);
                        if my_message_count == 0 {
                            break;
                        }
                    }
                }
            };
        }
    });

    let mut my_rooms = vec![];
    for i in 0..config.rooms_to_join {
        let duration = tokio::time::Duration::from_millis(config.time_between_messages);
        tokio::time::sleep(duration).await;

        let idx = (idx + i) % rooms.len();
        let room = &rooms[idx];
        my_rooms.push(room);

        let msg = format!("JOIN {}", room);
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(msg))
            .await?;
    }

    for i in 0..config.messages_to_send {
        let duration = tokio::time::Duration::from_millis(config.time_between_messages);
        tokio::time::sleep(duration).await;

        let room = my_rooms[(idx + i) % my_rooms.len()];
        let msg = format!("MSG {} {}: hello {}", room, idx, room);
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(msg))
            .await?;
    }

    reader.await?;
    write.close().await?;

    let time_taken = now.elapsed().as_millis();

    return Ok((
        time_taken as usize,
        msg_count.load(std::sync::atomic::Ordering::Relaxed),
        timeout.load(std::sync::atomic::Ordering::Relaxed),
    ));
}

enum Run {
    Error,
    Timeout,
    Success(usize, usize),
}

pub async fn client() -> Result<()> {
    dotenv().ok();

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

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Option<Run>>(config.parallel);

    let rx_handle = tokio::spawn(async move {
        let mut results = vec![];
        let mut timeouts = 0;
        let mut errors = 0;
        while let Some(run) = rx.recv().await {
            match run {
                Some(Run::Error) => {
                    errors += 1;
                }
                Some(Run::Timeout) => {
                    timeouts += 1;
                }
                Some(Run::Success(time_taken, count)) => {
                    results.push((time_taken, count));
                }
                None => {
                    break;
                }
            }
        }

        println!("time_taken, count");
        for (time_taken, count) in results {
            println!("{}, {}", time_taken, count);
        }
        println!("{} clients timed out", timeouts);
        println!("{} clients errored", errors);
    });

    let mut handles = vec![];
    for i in 0..config.count {
        if i < config.parallel {
            tokio::time::sleep(tokio::time::Duration::from_millis(config.time_between_connections))
                .await;
        }

        let permit = semaphore.clone().acquire_owned().await;
        let tx = tx.clone();

        if (i + 1) % 1000 == 0 {
            println!("{} clients spawned", i + 1);
        }

        let handle = tokio::spawn(async move {
            match run_client(url, rooms, i, config).await {
                Ok((time_taken, count, timedout)) => {
                    if timedout {
                        _ = tx.send(Some(Run::Timeout)).await;
                    } else {
                        _ = tx.send(Some(Run::Success(time_taken, count))).await;
                    }
                }
                Err(_) => {
                    _ = tx.send(Some(Run::Error)).await;
                }
            };

            drop(permit);
        });

        if handles.len() < config.parallel {
            handles.push(handle);
        } else {
            handles[i % config.parallel] = handle;
        }
    }

    futures_util::future::join_all(handles).await;
    _ = tx.send(None).await;
    rx_handle.await?;

    match reqwest::get(&format!("http://{}:{}/stop", config.host, config.stop_port)).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to stop server: {:?}", e);
        }
    }

    return Ok(());
}
