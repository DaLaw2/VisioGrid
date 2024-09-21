use crate::management::agent_manager::AgentManager;
use crate::management::monitor::Monitor;
use crate::utils::config::Config;
use actix::prelude::*;
use actix_web_actors::ws;
use serde_json;
use std::time::Duration;
use uuid::Uuid;

pub struct PerformanceWebSocket {
    pub target_type: String,
    pub agent_id: Option<Uuid>,
    pub interval: Option<SpawnHandle>,
}

impl Actor for PerformanceWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let future = async {
            let config = Config::now().await;
            Duration::from_secs(config.refresh_interval)
        };
        ctx.wait(future.into_actor(self).map(|interval, actor, ctx| {
            let handle = ctx.run_interval(interval, |act, ctx| {
                let target_type = act.target_type.clone();
                let agent_id = act.agent_id.clone();
                let future = async move {
                    if target_type == "system" {
                        Some(Monitor::get_performance().await)
                    } else {
                        match agent_id {
                            Some(agent_id) => AgentManager::get_agent_performance(agent_id).await,
                            None => None,
                        }
                    }
                };
                ctx.wait(future.into_actor(act).map(|performance, _, ctx| {
                    match performance {
                        Some(performance) => {
                            match serde_json::to_string(&performance) {
                                Ok(json) => ctx.text(json),
                                Err(_) => ctx.stop(),
                            }
                        },
                        None => ctx.stop(),
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

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for PerformanceWebSocket {
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
