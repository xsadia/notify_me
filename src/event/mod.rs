use core::fmt;

use chrono::{DateTime, Local, Utc};
use rusqlite::{
    types::{FromSql, ToSqlOutput},
    ToSql,
};

#[derive(Debug)]
pub enum RecurrencePattern {
    Daily,
    Weekly,
    Monthly,
}

impl FromSql for RecurrencePattern {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match String::column_result(value) {
            Ok(recurrence_pattern) => match recurrence_pattern.as_str() {
                "daily" => Ok(RecurrencePattern::Daily),
                "weekly" => Ok(RecurrencePattern::Weekly),
                "monthly" => Ok(RecurrencePattern::Monthly),
                _ => Err(rusqlite::types::FromSqlError::Other(Box::new(
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "unexpected value"),
                ))),
            },
            Err(err) => Err(rusqlite::types::FromSqlError::Other(Box::new(err))),
        }
    }
}

impl ToSql for RecurrencePattern {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match *self {
            RecurrencePattern::Daily => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Text(String::from("daily")),
            )),
            RecurrencePattern::Weekly => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Text(String::from("weekly")),
            )),
            RecurrencePattern::Monthly => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Text(String::from("monthly")),
            )),
        }
    }
}

#[derive(Debug)]
pub struct Event {
    #[allow(unused)]
    pub id: i32,
    pub name: String,
    pub message: String,
    pub recurrence_pattern: Option<RecurrencePattern>,
    pub date: DateTime<Local>,
    #[allow(unused)]
    pub deleted_at: Option<DateTime<Utc>>,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recurrence = match self.recurrence_pattern {
            Some(RecurrencePattern::Daily) => "daily",
            Some(RecurrencePattern::Weekly) => "weekly",
            Some(RecurrencePattern::Monthly) => "monthly",
            None => "once",
        };

        write!(
            f,
            "Event: {}\nAt: {}\nRecurrence: {}",
            self.name,
            self.date.format("%Y-%m-%d %H:%M"),
            recurrence,
        )
    }
}

pub struct EventList(pub Vec<Event>);

impl fmt::Display for EventList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            write!(f, "No events today")?;
        } else {
            for (i, event) in self.0.iter().enumerate() {
                if i > 0 {
                    write!(f, "\n\n")?;
                }
                write!(f, "{}", event)?;
            }
        }

        Ok(())
    }
}
