use std::time::Duration;
use actix::prelude::*;
use actix_web_actors::ws;
use crate::management::monitor::Monitor;
use crate::utils::config::Config;
use serde_json;

pub struct WebSocket {
    pub interval: Option<SpawnHandle>
}

impl Actor for WebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let refresh_interval_future = async {
            let config = Config::now().await;
            Duration::from_secs(config.refresh_interval)
        };
        ctx.wait(refresh_interval_future.into_actor(self).map(move |interval, actor, ctx| {
            let handle = ctx.run_interval(interval, move |act, ctx| {
                let performance_future = async {
                    let performance = Monitor::get_performance().await;
                    serde_json::to_string(&performance)
                };
                ctx.wait(performance_future.into_actor(act).map(|result, _, ctx| {
                    if let Ok(json) = result {
                        ctx.text(json);
                    }
                }));
            });
            actor.interval = Some(handle);
        }));
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        if let Some(interval) = self.interval.take() {
            ctx.cancel_future(interval);
        }
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => (),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => ctx.close(reason),
            _ => (),
        }
    }
}
