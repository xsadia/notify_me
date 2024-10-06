use chrono::{DateTime, Datelike, Duration, Local, Utc};
use log::{error, info};
use notify_rust::Notification;
use rusqlite::Connection;

use crate::event::{Event, RecurrencePattern};

#[cfg(target_os = "macos")]
static SOUND: &str = "Submarine";

#[cfg(all(unix, not(target_os = "macos")))]
static SOUND: &str = "message-new-instant";

#[cfg(target_os = "windows")]
static SOUND: &str = "Mail";

pub struct Scheduler<'a> {
    conn: &'a Connection,
}

impl<'a> Scheduler<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn check_and_notify(&self) -> Result<(), String> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, name, message, recurrence_pattern, date, deleted_at FROM events \
           WHERE (strftime('%Y-%m-%d %H:%M', date) = strftime('%Y-%m-%d %H:%M', 'now') \
           OR strftime('%Y-%m-%d %H:%M', date) = strftime('%Y-%m-%d %H:%M', datetime('now', '+10 minutes')))
           AND deleted_at IS NULL;",
        ) {
            Ok(stmt) => stmt,
            Err(err) => return Err(err.to_string()),
        };

        let events = match stmt.query_map([], |row| {
            Ok(Event {
                id: row.get(0)?,
                name: row.get(1)?,
                message: row.get(2)?,
                recurrence_pattern: row.get(3)?,
                date: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap()
                    .with_timezone(&Local),
                deleted_at: row.get::<_, Option<String>>(5)?.and_then(|dt| {
                    DateTime::parse_from_rfc3339(&dt)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
            })
        }) {
            Ok(events) => events
                .filter_map(|event| event.ok())
                .collect::<Vec<Event>>(),
            Err(err) => return Err(err.to_string()),
        };

        for event in events {
            match Notification::new()
                .summary(&event.name)
                .sound_name(SOUND)
                .body(&event.message)
                .icon("computer")
                .show()
            {
                Ok(_) => (),
                Err(err) => return Err(err.to_string()),
            }

            match event.recurrence_pattern {
                RecurrencePattern::Once => (),
                _ => {
                    match self.update_event_date(event) {
                        Ok(_) => return Ok(()),
                        Err(err) => return Err(err.to_string()),
                    };
                }
            };
        }

        Ok(())
    }

    fn update_event_date(&self, event: Event) -> Result<(), String> {
        let mut stmt = match self
            .conn
            .prepare("UPDATE events SET date = ?1 WHERE id = ?2;")
        {
            Ok(stmt) => stmt,
            Err(err) => return Err(err.to_string()),
        };

        let new_date = match event.recurrence_pattern {
            RecurrencePattern::Daily => event.date + Duration::days(1),
            RecurrencePattern::Weekly => event.date + Duration::weeks(1),
            RecurrencePattern::Monthly => {
                let next_month = event.date.month() % 12 + 1; // wraps around after December
                let next_year = if next_month == 1 {
                    event.date.year() + 1
                } else {
                    event.date.year()
                };

                event
                    .date
                    .with_year(next_year)
                    .unwrap()
                    .with_month(next_month)
                    .unwrap_or(event.date)
            }
            _ => unreachable!(),
        };

        match stmt.execute((new_date.to_rfc3339(), event.id)) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }

    pub async fn start(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

        loop {
            interval.tick().await;
            info!("Starting tick");
            if let Err(err) = self.check_and_notify() {
                error!("{}", err);
            } else {
                info!("Successfully ticked")
            }
        }
    }
}
