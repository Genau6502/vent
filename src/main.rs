#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::items_after_statements)]

mod error;
mod liquid_utils;
mod routes;

use axum::{
    routing::{get, post},
    Router,
};
use liquid_utils::partials::{init_partials, PARTIALS};
use routes::{
    add_event::{self, get_add_event_form, post_add_event_form},
    add_people_to_event::{
        self, get_add_people_to_event, post_add_participants_to_event, post_add_prefects_to_event,
    },
    add_person::{self, get_add_person, post_add_person},
    index::{self, get_index},
};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, sync::Arc};

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("unable to get env variables");
    tracing_subscriber::fmt::init();

    PARTIALS
        .set(init_partials().await)
        .expect("unable to set partials");

    let db_url = std::env::var("DATABASE_URL").expect("DB URL must be set");
    let pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .expect("cannot connect to DB"),
    );

    let app = Router::new()
        .route(index::LOCATION, get(get_index))
        .route(
            add_event::LOCATION,
            get(get_add_event_form).post(post_add_event_form),
        )
        .route(add_people_to_event::LOCATION, get(get_add_people_to_event))
        .route("/add_participant", post(post_add_participants_to_event))
        .route("/add_prefect", post(post_add_prefects_to_event))
        .route(
            add_person::LOCATION,
            get(get_add_person).post(post_add_person),
        )
        .with_state(pool);

    axum::Server::bind(&SocketAddr::from(([127, 0, 0, 1], 8080)))
        .serve(app.into_make_service())
        .await
        .unwrap();
}
