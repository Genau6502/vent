use crate::{error::KnotError, liquid_utils::compile, routes::Person};
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::Form;
use chrono::NaiveDateTime;
use serde::Deserialize;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

use super::DbEvent;

pub const LOCATION: &str = "/add_event";

pub async fn get_add_event_form(
    State(pool): State<Arc<Pool<Postgres>>>,
) -> Result<impl IntoResponse, KnotError> {
    let mut conn = pool.acquire().await?;

    let prefects: Vec<Person> = sqlx::query_as!(
        Person,
        r#"
SELECT person_name, id, is_prefect
FROM people
WHERE people.is_prefect = TRUE
        "#
    )
    .fetch_all(&mut conn)
    .await?;

    let globals = liquid::object!({ "prefects": prefects });

    info!("here");

    compile("www/add_event.liquid", globals).await
}

#[derive(Debug, Deserialize)]
pub struct FormEvent {
    pub name: String,
    pub date: String,
    pub location: String,
    pub teacher: String,
    pub info: String,
}

impl TryFrom<FormEvent> for DbEvent {
    type Error = KnotError;

    fn try_from(
        FormEvent {
            name,
            date,
            location,
            teacher,
            info,
        }: FormEvent,
    ) -> Result<Self, Self::Error> {
        let date = NaiveDateTime::parse_from_str(&date, "%Y-%m-%dT%H:%M")?;

        Ok(Self {
            id: -1,
            event_name: name,
            date,
            location,
            teacher,
            other_info: Some(info),
        })
    }
}

pub async fn post_add_event_form(
    State(pool): State<Arc<Pool<Postgres>>>,
    Form(event): Form<FormEvent>,
) -> Result<impl IntoResponse, KnotError> {
    info!(?event);
    let mut conn = pool.acquire().await?;

    let DbEvent {
        id: _,
        event_name,
        date,
        location,
        teacher,
        other_info: info,
    } = DbEvent::try_from(event)?;

    sqlx::query!(
        r#"
INSERT INTO public.events
(event_name, "date", "location", teacher, other_info)
VALUES($1, $2, $3, $4, $5)
RETURNING id
        "#,
        event_name,
        date,
        location,
        teacher,
        info
    )
    .fetch_one(&mut conn)
    .await?;

    Ok(Redirect::to(LOCATION))
}