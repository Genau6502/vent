pub mod add_event;
pub mod add_people_to_event;
pub mod add_person;
pub mod index;
pub mod remove_stuff;
pub mod calendar;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Person {
    pub person_name: String,
    pub is_prefect: bool,
    pub id: i32,
}

#[derive(Deserialize)]
pub struct DbEvent {
    pub id: i32,
    pub event_name: String,
    pub date: NaiveDateTime,
    pub location: String,
    pub teacher: String,
    pub other_info: Option<String>,
}
