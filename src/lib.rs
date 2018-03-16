//! Stores is a microservice responsible for authentication and managing stores and products
//! The layered structure of the app is
//!
//! `Application -> Controller -> Service -> Repo + HttpClient`
//!
//! Each layer can only face exceptions in its base layers and can only expose its own errors.
//! E.g. `Service` layer will only deal with `Repo` and `HttpClient` errors and will only return
//! `ServiceError`. That way Controller will only have to deal with ServiceError, but not with `Repo`
//! or `HttpClient` repo.

extern crate chrono;
extern crate config as config_crate;
#[macro_use]
extern crate diesel;
extern crate elastic_responses;
extern crate elastic_types;
#[macro_use]
extern crate elastic_types_derive;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate hyper_tls;
extern crate isolang;
extern crate jsonwebtoken;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate stq_acl;
extern crate stq_http;
extern crate stq_router;
extern crate stq_static_resources;
extern crate tokio_core;
extern crate validator;
#[macro_use]
extern crate validator_derive;

#[macro_use]
pub mod macros;
pub mod controller;
pub mod models;
pub mod repos;
pub mod elastic;
pub mod services;
pub mod config;
pub mod types;

use std::sync::Arc;
use std::process;

use futures::{Future, Stream};
use futures::future;
use futures_cpupool::CpuPool;
use hyper::server::Http;
use diesel::pg::PgConnection;
use r2d2_diesel::ConnectionManager;
use tokio_core::reactor::Core;

use stq_http::controller::Application;
use stq_http::client::Config as HttpConfig;

use config::Config;
use repos::acl::RolesCacheImpl;
use repos::categories::CategoryCacheImpl;
use repos::attributes::AttributeCacheImpl;

/// Starts new web service from provided `Config`
pub fn start_server(config: Config) {
    // Prepare logger
    env_logger::init().unwrap();

    // Prepare reactor
    let mut core = Core::new().expect("Unexpected error creating event loop core");
    let handle = Arc::new(core.handle());

    let http_config = HttpConfig {
        http_client_retries: config.client.http_client_retries,
        http_client_buffer_size: config.client.http_client_buffer_size,
    };
    let client = stq_http::client::Client::new(&http_config, &handle);
    let client_handle = client.handle();
    let client_stream = client.stream();
    handle.spawn(client_stream.for_each(|_| Ok(())));

    // Prepare database pool
    let database_url: String = config
        .server
        .database
        .parse()
        .expect("Database URL must be set in configuration");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let r2d2_pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create connection pool");

    let thread_count = config.server.thread_count.clone();

    // Prepare CPU pool
    let cpu_pool = CpuPool::new(thread_count);

    // Prepare server
    let address = config
        .server
        .address
        .parse()
        .expect("Address must be set in configuration");

    // Roles cache
    let roles_cache = RolesCacheImpl::default();

    // Categories cache
    let category_cache = CategoryCacheImpl::default();

    // Attributes cache
    let attributes_cache = AttributeCacheImpl::default();

    // Controller
    let controller = controller::ControllerImpl::new(
        r2d2_pool,
        cpu_pool,
        client_handle,
        config,
        roles_cache,
        category_cache,
        attributes_cache,
    );

    let serve = Http::new()
        .serve_addr_handle(&address, &handle, move || {
            let controller = Box::new(controller.clone());

            // Prepare application
            let app = Application { controller };

            Ok(app)
        })
        .unwrap_or_else(|why| {
            error!("Http Server Initialization Error: {}", why);
            process::exit(1);
        });

    let handle_arc2 = handle.clone();
    handle.spawn(
        serve
            .for_each(move |conn| {
                handle_arc2.spawn(
                    conn.map(|_| ())
                        .map_err(|why| error!("Server Error: {:?}", why)),
                );
                Ok(())
            })
            .map_err(|_| ()),
    );

    info!("Listening on http://{}, threads: {}", address, thread_count);
    core.run(future::empty::<(), ()>()).unwrap();
}
