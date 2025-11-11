use std::time::{Duration, Instant};

use actix_ws::Message;
use futures_util::{
    StreamExt as _,
    future::{self, Either},
};
use tokio::{pin, time::interval};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn echo_heartbeat_ws(
    mut session: actix_ws::Session,
    mut msg_stream: actix_ws::MessageStream,
) {
    log::info!("connected");

    let mut last_heartbeat = Instant::now();
    let mut interval = interval(HEARTBEAT_INTERVAL);

    let reason = loop {
        let tick = interval.tick();
        pin!(tick);

        match future::select(msg_stream.next(), tick).await {
            Either::Left((Some(Ok(msg)), _)) => {
                log::debug!("msg: {msg:?}");

                match msg {
                    Message::Text(text) => {
                        session.text(text).await.unwrap();
                    }

                    Message::Binary(bin) => {
                        session.binary(bin).await.unwrap();
                    }

                    Message::Close(reason) => {
                        break reason;
                    }

                    Message::Ping(bytes) => {
                        last_heartbeat = Instant::now();
                        let _ = session.pong(&bytes).await;
                    }

                    Message::Pong(_) => {
                        last_heartbeat = Instant::now();
                    }

                    Message::Continuation(_) => {
                        log::warn!("no support for continuation frames");
                    }

                    Message::Nop => {}
                };
            }

            Either::Left((Some(Err(err)), _)) => {
                log::error!("{}", err);
                break None;
            }

            Either::Left((None, _)) => break None,

            Either::Right((_inst, _)) => {
                if Instant::now().duration_since(last_heartbeat) > CLIENT_TIMEOUT {
                    log::info!(
                        "client has not sent heartbeat in over {CLIENT_TIMEOUT:?}; disconnecting"
                    );

                    break None;
                }

                let _ = session.ping(b"").await;
            }
        }
    };

    let _ = session.close(reason).await;

    log::info!("disconnected");
}
