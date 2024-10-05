use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use rusqlite::Connection;

use crate::event::{Event, EventList, RecurrencePattern};

pub struct Client<'a> {
    conn: &'a Connection,
}

impl<'a> Client<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn start(&self) {
        let selections = &["Today", "Create", "Update", "Delete"];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an action")
            .default(0)
            .items(&selections[..])
            .interact()
            .unwrap();

        match selections[selection] {
            "Today" => println!("{}", self.fetch_current_day_events().unwrap()),
            "Create" => self.create_event().unwrap(),
            _ => todo!(),
        }
    }

    fn create_event(&self) -> Result<(), String> {
        let mut stmt = match self.conn.prepare(
            "INSERT INTO EVENTS (name, message, recurrence_pattern, date) VALUES (?1, ?2, ?3, ?4)",
        ) {
            Ok(stmt) => stmt,
            Err(err) => return Err(err.to_string()),
        };

        let event_name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Event name")
            .validate_with(move |input: &String| -> Result<(), &str> {
                if !input.is_empty() {
                    Ok(())
                } else {
                    Err("Event name can't be empty")
                }
            })
            .interact_text()
            .unwrap();

        let event_description: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Event description")
            .allow_empty(true)
            .interact_text()
            .unwrap();

        let date_format = "%d/%m/%Y %H:%M";
        let event_date_input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Event date (dd/mm/yyyy hh:mm)")
            .validate_with({
                move |input: &String| -> Result<(), &str> {
                    if NaiveDateTime::parse_from_str(input, date_format).is_ok() {
                        Ok(())
                    } else {
                        Err("Invalid date format. Please use 'dd/mm/yyyy hh:mm'")
                    }
                }
            })
            .interact_text()
            .unwrap();

        let event_date: DateTime<Utc> = {
            let naive_date = NaiveDateTime::parse_from_str(&event_date_input, date_format)
                .expect("Failed to parse date");
            let local = Local.from_local_datetime(&naive_date).unwrap();
            local.with_timezone(&Utc)
        };

        let recurrence: Option<RecurrencePattern> =
            match Input::<String>::with_theme(&ColorfulTheme::default())
                .with_prompt("Event recurrence")
                .allow_empty(true)
                .interact_text()
                .unwrap()
                .as_str()
                .trim()
            {
                "daily" => Some(RecurrencePattern::Daily),
                "weekly" => Some(RecurrencePattern::Weekly),
                "monthly" => Some(RecurrencePattern::Monthly),
                _ => None,
            };

        match stmt.execute((
            event_name,
            event_description,
            recurrence,
            event_date.with_timezone(&Local).to_rfc3339(),
        )) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }

    fn fetch_current_day_events(&self) -> Result<EventList, String> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, name, message, recurrence_pattern, date, deleted_at FROM events \
       WHERE strftime('%Y-%m-%d', date) = strftime('%Y-%m-%d', 'now') \
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
                recurrence_pattern: row.get(3).ok(),
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

        Ok(EventList(events))
    }
}
