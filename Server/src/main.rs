#![allow(non_snake_case)]

use tokio::time::sleep;
use std::time::Duration;
use library::web::page::log;
use library::web::page::config;
use library::web::server::Server;
use library::web::page::inference;
use library::web::page::javascript;
use library::utils::config::Config;
use actix_web::{App, Error, HttpServer};
use library::utils::logger::{Logger, LogLevel};
use library::manager::file_manager::FileManager;
use library::manager::node_cluster::NodeCluster;

#[actix_web::main]
async fn main() -> Result<(), Error> {
    Config::now().await;
    FileManager::run().await;
    NodeCluster::run().await;
    Server::run().await;
    let http_server = loop {
        let config = Config::now().await;
        let http_server = HttpServer::new(|| {
            App::new()
                .service(config::initialize())
                .service(inference::initialize())
                .service(log::initialize())
                .service(javascript::initialize())
        }).bind(format!("127.0.0.1:{}", config.http_server_bind_port));
        match http_server {
            Ok(http_server) => break http_server,
            Err(err) => {
                Logger::append_system_log(LogLevel::ERROR, format!("Http Server: Bind port failed.\nReason: {}", err)).await;
                sleep(Duration::from_millis(config.internal_timestamp)).await;
                continue;
            },
        }
    };
    Logger::append_system_log(LogLevel::INFO, "Http Server: Online.".to_string()).await;
    match http_server.run().await {
        Ok(_) => {},
        Err(err) => Logger::append_system_log(LogLevel::ERROR, format!("Http Server: Internal server error.\nReason: {}", err)).await,
    };
    FileManager::terminate().await;
    NodeCluster::terminate().await;
    Server::terminate().await;
    Ok(())
}
