use std::time::Duration;
use actix::prelude::*;
use actix_web_actors::ws;
use serde_json;
use uuid::Uuid;
use crate::management::agent_manager::AgentManager;
use crate::management::monitor::Monitor;
use crate::utils::config::Config;

pub struct PerformanceWebSocket {
    pub target_type: String,
    pub agent_id: Option<Uuid>,
    pub interval: Option<SpawnHandle>,
}
impl Actor for PerformanceWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let refresh_interval_future = async {
            let config = Config::now().await;
            Duration::from_secs(config.refresh_interval)
        };
        ctx.wait(refresh_interval_future.into_actor(self).map(move |interval, actor, ctx| {
            let handle = ctx.run_interval(interval, move |act, ctx| {
                let performance_future = async {
                    match act.target_type.as_str() {
                        "system" => {
                            let performance = Monitor::get_system_performance().await;
                            serde_json::to_string(&performance).map_err(|_| ())
                        },
                        _ => {
                            if let Some(agent_id) = act.agent_id {
                                let performance = AgentManager::get_agent_performance(agent_id).await;
                                serde_json::to_string(&performance).map_err(|_| ())
                            } else {
                                Err(())
                            }
                        }
                    }
                };
                ctx.wait(performance_future.into_actor(act).map(|result, _, ctx| {
                    match result {
                        Ok(json) => ctx.text(json),
                        Err(_) => ctx.stop(),
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
