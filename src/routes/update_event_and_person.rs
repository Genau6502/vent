use super::FormEvent;
use crate::{
    error::KnotError,
    liquid_utils::compile,
    routes::{DbEvent, DbPerson},
};
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::Form;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use tokio::fs::remove_file;
use std::{sync::Arc, collections::HashMap};

#[allow(clippy::too_many_lines)]
pub async fn get_update_event(
    Path(event_id): Path<i32>,
    State(pool): State<Arc<Pool<Postgres>>>,
) -> Result<impl IntoResponse, KnotError> {
    let mut conn = pool.acquire().await?;

    let DbEvent {
        id,
        event_name,
        date,
        location,
        teacher,
        other_info,
    } = sqlx::query_as!(
        DbEvent,
        r#"
SELECT * FROM events WHERE id = $1
"#,
        event_id
    )
    .fetch_one(&mut conn)
    .await?;
    let date = date.to_string();

    #[derive(Deserialize, Serialize, Debug, Clone)]
    struct PersonPlusRelID {
        pub id: i32,
        pub first_name: String,
        pub surname: String,
        pub form: String,
        pub relation_id: i32,
    }

    #[derive(Serialize, Clone)]
    struct RelFormGroup {
        pub form: String,
        pub people: Vec<PersonPlusRelID>
    }
    #[derive(Serialize, Clone)]
    struct DbFormGroup {
        pub form: String,
        pub people: Vec<DbPerson>
    }

    let mut existing_prefects = HashMap::new();
    for person in sqlx::query_as!(
        PersonPlusRelID,
        r#"
SELECT p.first_name, p.surname, pe.relation_id, p.id, p.form
FROM people p
INNER JOIN prefect_events pe ON pe.event_id = $1 AND pe.prefect_id = p.id
ORDER BY p.form
"#,
        event_id
    )
    .fetch_all(&mut conn)
    .await? {
        existing_prefects.entry(person.form.clone()).or_insert(RelFormGroup {
            form: person.form.clone(),
            people: vec![]
        }).people.push(person);
    }
    let existing_prefects = existing_prefects.into_values().collect::<Vec<_>>();

    let mut existing_participants = HashMap::new();
    for person in sqlx::query_as!(
        PersonPlusRelID,
        r#"
SELECT p.first_name, p.surname, pe.relation_id, p.id, p.form
FROM people p
INNER JOIN participant_events pe ON pe.event_id = $1 AND pe.participant_id = p.id
ORDER BY p.form
"#,
        event_id
    )
    .fetch_all(&mut conn)
    .await? {
        existing_participants.entry(person.form.clone()).or_insert(RelFormGroup {
            form: person.form.clone(),
            people: vec![]
        }).people.push(person);
    }
    let existing_participants = existing_participants.into_values().collect::<Vec<_>>();

    let mut possible_prefects = HashMap::new();
    for person in sqlx::query_as!(
        DbPerson,
        r#"
SELECT *
FROM people p
WHERE p.is_prefect = true
ORDER BY p.form
"#
    )
    .fetch_all(&mut conn)
    .await?
    .into_iter()
    .filter(|p| !existing_prefects.iter().any(|g| g.people.iter().any(|e| e.id == p.id))) {
        possible_prefects.entry(person.form.clone()).or_insert(DbFormGroup {
            form: person.form.clone(),
            people: vec![]
        }).people.push(person);
    }
    let possible_prefects = possible_prefects.into_values().collect::<Vec<_>>();

    let mut possible_participants = HashMap::new();
    for person in sqlx::query_as!(
        DbPerson,
        r#"
SELECT *
FROM people p
ORDER BY p.form
"#
    )
    .fetch_all(&mut conn)
    .await?
    .into_iter()
    .filter(|p| !existing_participants.iter().any(|g| g.people.iter().any(|e| e.id == p.id))) {
        possible_participants.entry(person.form.clone()).or_insert(DbFormGroup {
            form: person.form.clone(),
            people: vec![]
        }).people.push(person);
    }
    let possible_participants = possible_participants.into_values().collect::<Vec<_>>();

    #[derive(Serialize)]
    struct Image {
        path: String,
        id: i32,
    }

    let photos: Vec<Image> = sqlx::query_as!(
        Image,
        r#"
SELECT path, id FROM photos
WHERE event_id = $1
        "#,
        event_id
    )
    .fetch_all(&mut conn)
    .await?;

    let globals = liquid::object!({
        "event": liquid::object!({
            "id": id,
            "event_name": event_name,
            "date": date.to_string(),
            "location": location,
            "teacher": teacher,
            "other_info": other_info.unwrap_or_default()
        }),
        "existing_prefects": existing_prefects,
        "existing_participants": existing_participants,
        "prefects": possible_prefects,
        "participants": possible_participants,
        "n_imgs": photos.len(),
        "imgs": photos
    });

    compile("www/update_event.liquid", globals).await
}
pub async fn post_update_event(
    Path(event_id): Path<i32>,
    State(pool): State<Arc<Pool<Postgres>>>,
    Form(event): Form<FormEvent>,
) -> Result<impl IntoResponse, KnotError> {
    let mut conn = pool.acquire().await?;

    let DbEvent {
        id: _id,
        event_name,
        date,
        location,
        teacher,
        other_info,
    } = DbEvent::try_from(event)?;
    let other_info = other_info.unwrap_or_default();

    sqlx::query!(
        r#"
UPDATE public.events
SET event_name=$2, date=$3, location=$4, teacher=$5, other_info=$6
WHERE id=$1
        "#,
        event_id,
        event_name,
        date,
        location,
        teacher,
        other_info
    )
    .execute(&mut conn)
    .await?;

    Ok(Redirect::to(&format!("/update_event/{event_id}")))
}

pub async fn get_remove_prefect_from_event(
    Path(relation_id): Path<i32>,
    State(pool): State<Arc<Pool<Postgres>>>,
) -> Result<impl IntoResponse, KnotError> {
    let mut conn = pool.acquire().await?;

    let id = sqlx::query!(
        r#"
DELETE FROM prefect_events WHERE relation_id = $1 
RETURNING event_id
"#,
        relation_id
    )
    .fetch_one(&mut conn)
    .await?
    .event_id;

    Ok(Redirect::to(&format!("/update_event/{id}")))
}
pub async fn get_remove_participant_from_event(
    Path(relation_id): Path<i32>,
    State(pool): State<Arc<Pool<Postgres>>>,
) -> Result<impl IntoResponse, KnotError> {
    let mut conn = pool.acquire().await?;

    let id = sqlx::query!(
        r#"
DELETE FROM participant_events WHERE relation_id = $1 
RETURNING event_id
"#,
        relation_id
    )
    .fetch_one(&mut conn)
    .await?
    .event_id;

    Ok(Redirect::to(&format!("/update_event/{id}")))
}

pub async fn delete_image (Path(img_id): Path<i32>, State(pool): State<Arc<Pool<Postgres>>>) -> Result<impl IntoResponse, KnotError> {
    let mut conn = pool.acquire().await?;
    let event = sqlx::query!(
        r#"
DELETE FROM public.photos
WHERE id=$1
RETURNING path, event_id"#,
        img_id
    ).fetch_one(&mut conn).await?;

    remove_file(event.path).await?;

    Ok(Redirect::to(&format!("/update_event/{}", event.event_id)))
}