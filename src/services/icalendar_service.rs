use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use icalendar::{Calendar as ICalCalendar, CalendarComponent, Component, Todo};

use crate::{
    errors::{Result, TasksError},
    models::task::Task,
};

pub struct ICalendarService;

/// Représentation d'une VTODO importée (avant résolution des liens parent/labels).
#[derive(Debug, Clone)]
pub struct ParsedVtodo {
    pub uid:              String,
    pub summary:          String,
    pub description:      Option<String>,
    pub status:           String,        // open|in_progress|done|cancelled
    pub priority:         i16,
    pub percent_complete: i16,
    pub due_at:           Option<DateTime<Utc>>,
    pub start_at:         Option<DateTime<Utc>>,
    pub completed_at:     Option<DateTime<Utc>>,
    pub parent_uid:       Option<String>,
    pub categories:       Vec<String>,
    pub rrule:            Option<String>,
}

fn fmt_utc(dt: &DateTime<Utc>) -> String {
    dt.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Parse une valeur de date/heure iCalendar en DateTime<Utc>.
fn parse_ical_dt(raw: &str) -> Option<DateTime<Utc>> {
    // Retirer un éventuel préfixe de paramètres "TZID=...:" ou "VALUE=DATE:".
    let val = raw.rsplit(':').next().unwrap_or(raw).trim();
    if let Ok(dt) = NaiveDateTime::parse_from_str(val, "%Y%m%dT%H%M%SZ") {
        return Some(Utc.from_utc_datetime(&dt));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(val, "%Y%m%dT%H%M%S") {
        return Some(Utc.from_utc_datetime(&dt));
    }
    if let Ok(d) = NaiveDate::parse_from_str(val, "%Y%m%d") {
        return Some(Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0)?));
    }
    None
}

fn status_to_ical(status: &str) -> &'static str {
    match status {
        "in_progress" => "IN-PROCESS",
        "done"        => "COMPLETED",
        "cancelled"   => "CANCELLED",
        _             => "NEEDS-ACTION",
    }
}

fn status_from_ical(status: &str) -> String {
    match status.to_uppercase().as_str() {
        "IN-PROCESS" => "in_progress",
        "COMPLETED"  => "done",
        "CANCELLED"  => "cancelled",
        _            => "open",
    }
    .to_string()
}

impl ICalendarService {
    /// Construit une VTODO icalendar à partir d'une tâche.
    fn todo_component(task: &Task, categories: &[String]) -> Todo {
        let mut todo = Todo::new();
        todo.uid(&task.ical_uid);
        todo.summary(&task.title);
        todo.timestamp(Utc::now());

        if let Some(ref desc) = task.description {
            todo.description(desc);
        }
        if let Some(ref due) = task.due_at {
            todo.add_property("DUE", fmt_utc(due));
        }
        if let Some(ref start) = task.start_at {
            todo.add_property("DTSTART", fmt_utc(start));
        }
        if let Some(ref completed) = task.completed_at {
            todo.add_property("COMPLETED", fmt_utc(completed));
        }
        todo.add_property("STATUS", status_to_ical(&task.status));
        todo.add_property("PERCENT-COMPLETE", task.percent_complete.to_string());
        if task.priority > 0 {
            todo.add_property("PRIORITY", task.priority.to_string());
        }
        todo.add_property("SEQUENCE", task.sequence.to_string());
        if let Some(ref rrule) = task.rrule {
            todo.add_property("RRULE", rrule.trim_start_matches("RRULE:"));
        }
        if let Some(parent) = task.parent_task_id {
            // RELATED-TO porte l'UID du parent ; ici on n'a que l'id interne,
            // le parent_uid réel est résolu côté handler si nécessaire.
            todo.add_property("RELATED-TO", parent.to_string());
        }
        if !categories.is_empty() {
            todo.add_property("CATEGORIES", categories.join(","));
        }
        todo
    }

    /// Sérialise une tâche en chaîne iCalendar (.ics) contenant une VTODO.
    pub fn task_to_ics(task: &Task, categories: &[String]) -> String {
        let mut cal = ICalCalendar::new();
        cal.name("Kubuno Tasks");
        cal.push(Self::todo_component(task, categories));
        cal.to_string()
    }

    /// Sérialise un board entier (liste de tâches) en .ics.
    pub fn board_to_ics(board_title: &str, tasks: &[Task]) -> String {
        let mut cal = ICalCalendar::new();
        cal.name(board_title);
        for task in tasks {
            cal.push(Self::todo_component(task, &[]));
        }
        cal.to_string()
    }

    /// Parse un flux iCalendar et extrait toutes les VTODO.
    pub fn parse_vtodo(ics_content: &str) -> Result<Vec<ParsedVtodo>> {
        let calendar: ICalCalendar = ics_content
            .parse()
            .map_err(|e: String| TasksError::Validation(format!("ICS invalide: {e}")))?;

        let mut todos = Vec::new();
        for component in &calendar.components {
            if let CalendarComponent::Todo(t) = component {
                let uid = t.get_uid().unwrap_or("").to_string();
                if uid.is_empty() {
                    continue;
                }
                let summary = t.get_summary().unwrap_or("Sans titre").to_string();
                let description = t.get_description().map(|s| s.to_string());
                let status = t
                    .property_value("STATUS")
                    .map(status_from_ical)
                    .unwrap_or_else(|| "open".to_string());
                let priority = t
                    .property_value("PRIORITY")
                    .and_then(|v| v.parse::<i16>().ok())
                    .map(|p| p.clamp(0, 9))
                    .unwrap_or(0);
                let percent_complete = t
                    .property_value("PERCENT-COMPLETE")
                    .and_then(|v| v.parse::<i16>().ok())
                    .map(|p| p.clamp(0, 100))
                    .unwrap_or(if status == "done" { 100 } else { 0 });
                let due_at = t.property_value("DUE").and_then(parse_ical_dt);
                let start_at = t.property_value("DTSTART").and_then(parse_ical_dt);
                let completed_at = t.property_value("COMPLETED").and_then(parse_ical_dt);
                let parent_uid = t.property_value("RELATED-TO").map(|s| s.to_string());
                let categories = t
                    .property_value("CATEGORIES")
                    .map(|s| s.split(',').map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect())
                    .unwrap_or_default();
                let rrule = t.property_value("RRULE").map(|s| s.to_string());

                todos.push(ParsedVtodo {
                    uid,
                    summary,
                    description,
                    status,
                    priority,
                    percent_complete,
                    due_at,
                    start_at,
                    completed_at,
                    parent_uid,
                    categories,
                    rrule,
                });
            }
        }
        Ok(todos)
    }
}
