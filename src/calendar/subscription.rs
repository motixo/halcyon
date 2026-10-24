//! Read-only calendar feeds. Subscription URLs are credentials, never config or logs.
use super::*;
use crate::data::{CalendarData, CalendarEvent};
use chrono::{DateTime, NaiveDate, Utc};
use ical::{parser::ical::component::IcalEvent, property::Property};
use rrule::{RRuleSet, Tz};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SecretText(pub String);
impl std::fmt::Debug for SecretText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Feed {
    choice: CalendarChoice,
    url: SecretText,
}
#[derive(Clone, Serialize, Deserialize)]
struct Feeds {
    feeds: Vec<Feed>,
}
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_EVENTS: usize = 10000;

fn url(value: &str) -> Result<reqwest::Url, Problem> {
    if value.len() > 8192 {
        return Err(Problem::FeedUrl);
    }
    let value = value.trim();
    let value = value
        .strip_prefix("webcal://")
        .map(|s| format!("https://{s}"))
        .unwrap_or_else(|| value.into());
    let u = reqwest::Url::parse(&value).map_err(|_| Problem::FeedUrl)?;
    if u.scheme() != "https"
        || !u.has_host()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.fragment().is_some()
    {
        return Err(Problem::FeedUrl);
    }
    Ok(u)
}
async fn download(value: &SecretText) -> Result<Vec<u8>, Problem> {
    let client = reqwest::Client::builder()
        .referer(false) // A redirected request must not disclose a private subscription URL.
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5
                || attempt.url().scheme() != "https"
                || !attempt.url().username().is_empty()
                || attempt.url().password().is_some()
            {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|_| Problem::Network)?;
    let mut response = client
        .get(url(&value.0)?)
        .header("Accept", "text/calendar")
        .send()
        .await
        .map_err(|_| Problem::Network)?;
    if !response.status().is_success() {
        return Err(Problem::InvalidFeed);
    }
    if response
        .content_length()
        .is_some_and(|n| n > MAX_BYTES as u64)
    {
        return Err(Problem::FeedLimit);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Problem::Network)? {
        if bytes.len().saturating_add(chunk.len()) > MAX_BYTES {
            return Err(Problem::FeedLimit);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
async fn parsed(feed: &Feed, month: &str) -> Result<Vec<CalendarEvent>, Problem> {
    let bytes = download(&feed.url).await?;
    let month = month.to_string();
    let choice = feed.choice.clone();
    tokio::task::spawn_blocking(move || parse(&bytes, &month, &choice))
        .await
        .map_err(|_| Problem::InvalidFeed)?
}
pub async fn add_subscription(
    expected: Connection,
    name: String,
    secret: SecretText,
) -> Result<Connection, Problem> {
    if store::preview() {
        return Err(Problem::Preview);
    }
    let normalized = SecretText(url(&secret.0)?.into());
    let mut feeds = if expected.backend == Backend::Subscription {
        keyring::read::<Feeds>(&expected.credential, true).await?
    } else {
        Feeds { feeds: vec![] }
    };
    if feeds.feeds.len() >= 8 {
        return Err(Problem::FeedLimit);
    }
    if feeds.feeds.iter().any(|f| f.url.0 == normalized.0) {
        return Err(Problem::InvalidFeed);
    }
    let name: String = name.trim().chars().take(80).collect();
    let choice = CalendarChoice {
        id: store::nonce()?,
        name: if name.is_empty() {
            "Calendar".into()
        } else {
            name
        },
        primary: false,
        color: String::new(),
    };
    let feed = Feed {
        choice: choice.clone(),
        url: normalized,
    };
    parsed(&feed, &chrono::Local::now().format("%Y-%m").to_string()).await?;
    keyring::ready().await?;
    feeds.feeds.push(feed);
    let selections = if expected.backend == Backend::Subscription {
        let mut s = expected.selected_calendars();
        s.push(choice.clone());
        s
    } else {
        vec![choice.clone()]
    };
    let connection = Connection {
        backend: Backend::Subscription,
        account: "Calendar subscriptions".into(),
        calendar_id: choice.id.clone(),
        calendar_name: choice.name.clone(),
        revision: store::nonce()?,
        credential: store::nonce()?,
        selections,
        subscriptions: feeds.feeds.iter().map(|f| f.choice.clone()).collect(),
    };
    commit(&expected, connection, &feeds).await
}
async fn commit(
    expected: &Connection,
    connection: Connection,
    feeds: &Feeds,
) -> Result<Connection, Problem> {
    keyring::save(&connection.credential, feeds).await?;
    if let Err(problem) = store::replace(expected, &connection) {
        auth::forget(&connection.credential).await;
        return Err(problem);
    }
    auth::forget(&expected.credential).await;
    Ok(connection)
}
pub async fn remove_subscription(expected: Connection, id: String) -> Result<Connection, Problem> {
    if store::preview() {
        return Err(Problem::Preview);
    }
    if expected.backend != Backend::Subscription {
        return Err(Problem::Changed);
    }
    let mut feeds: Feeds = keyring::read(&expected.credential, true).await?;
    let old = feeds.feeds.len();
    feeds.feeds.retain(|f| f.choice.id != id);
    if feeds.feeds.len() == old {
        return Err(Problem::Changed);
    }
    if feeds.feeds.is_empty() {
        return super::disconnect(expected).await;
    }
    let mut connection = expected.clone();
    connection.subscriptions = feeds.feeds.iter().map(|f| f.choice.clone()).collect();
    connection.selections = expected
        .selected_calendars()
        .into_iter()
        .filter(|c| c.id != id)
        .collect();
    if connection.selections.is_empty() {
        connection
            .selections
            .push(connection.subscriptions[0].clone());
    }
    connection.calendar_id = connection.selections[0].id.clone();
    connection.calendar_name = connection.selections[0].name.clone();
    connection.revision = store::nonce()?;
    connection.credential = store::nonce()?;
    commit(&expected, connection, &feeds).await
}
pub(super) async fn events(
    connection: &Connection,
    month: &str,
    unlock: bool,
) -> Result<CalendarData, Problem> {
    let feeds: Feeds = keyring::read(&connection.credential, unlock).await?;
    if feeds.feeds.len() > 8 {
        return Err(Problem::FeedLimit);
    }
    let mut events = vec![];
    for choice in connection.selected_calendars() {
        let feed = feeds
            .feeds
            .iter()
            .find(|f| f.choice.id == choice.id)
            .ok_or(Problem::Changed)?;
        events.extend(parsed(feed, month).await?);
        if events.len() > MAX_EVENTS
            || events.iter().map(event_text_bytes).sum::<usize>() > EVENT_TEXT_BUDGET
        {
            return Err(Problem::FeedLimit);
        }
    }
    events.sort_by(CalendarEvent::chronological_order);
    Ok(CalendarData {
        account: connection.account.clone(),
        month: month.into(),
        updated_at: Utc::now().to_rfc3339(),
        events,
        ..Default::default()
    })
}
fn value<'a>(props: &'a [Property], name: &str) -> Option<&'a str> {
    props.iter().find(|p| p.name == name)?.value.as_deref()
}
fn prop<'a>(props: &'a [Property], name: &str) -> Option<&'a Property> {
    props.iter().find(|p| p.name == name)
}
fn param<'a>(p: &'a Property, key: &str) -> Option<&'a str> {
    p.params
        .as_ref()?
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))?
        .1
        .first()
        .map(String::as_str)
}
fn unescape(text: &str, limit: usize) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    for _ in 0..limit {
        match chars.next() {
            Some('\\') => match chars.next() {
                Some('n' | 'N') => out.push('\n'),
                Some(c) => out.push(c),
                None => break,
            },
            Some(c) => out.push(c),
            None => break,
        }
    }
    out
}
fn datetime(p: &Property, raw: &str, fallback: Option<&str>) -> Result<DateTime<Tz>, Problem> {
    let zone = param(p, "TZID")
        .or(fallback)
        .filter(|_| !raw.ends_with('Z') && raw.len() != 8);
    if zone.is_some_and(|z| z.contains(['\r', '\n', ':', ';'])) {
        return Err(Problem::FeedTimezone);
    }
    let line = if let Some(z) = zone {
        format!("DTSTART;TZID={z}:{raw}")
    } else {
        format!("DTSTART:{raw}")
    };
    let set: RRuleSet = format!("{line}\nRRULE:FREQ=DAILY;COUNT=1")
        .parse()
        .map_err(|_| {
            if zone.is_some() {
                Problem::FeedTimezone
            } else {
                Problem::InvalidFeed
            }
        })?;
    Ok(*set.get_dt_start())
}
fn duration(text: &str) -> Result<(chrono::Duration, u64), Problem> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"^\+?P(?:(\d+)W)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?$")
            .unwrap()
    });
    let caps = re.captures(text).ok_or(Problem::InvalidFeed)?;
    let mut seconds = 0i64;
    let mut nominal_days = 0u64;
    if caps.iter().skip(1).all(|part| part.is_none()) {
        return Err(Problem::InvalidFeed);
    }
    for (i, scale) in [604800i64, 86400, 3600, 60, 1].into_iter().enumerate() {
        if let Some(m) = caps.get(i + 1) {
            let n = m.as_str().parse::<i64>().map_err(|_| Problem::FeedLimit)?;
            seconds = seconds
                .checked_add(n.checked_mul(scale).ok_or(Problem::FeedLimit)?)
                .ok_or(Problem::FeedLimit)?;
            if i < 2 {
                nominal_days = seconds as u64 / 86400;
            }
        }
    }
    if seconds > 366 * 86400 {
        return Err(Problem::FeedLimit);
    }
    Ok((chrono::Duration::seconds(seconds), nominal_days))
}
fn event(
    e: &IcalEvent,
    fallback: Option<&str>,
    choice: &CalendarChoice,
) -> Result<(DateTime<Tz>, chrono::Duration, u64, CalendarEvent), Problem> {
    let p = &e.properties;
    let start_prop = prop(p, "DTSTART").ok_or(Problem::InvalidFeed)?;
    let raw = start_prop.value.as_deref().ok_or(Problem::InvalidFeed)?;
    let start = datetime(start_prop, raw, fallback)?;
    let all_day = raw.len() == 8;
    let (span, nominal_days) = if let Some(end) = prop(p, "DTEND") {
        let raw_end = end.value.as_deref().ok_or(Problem::InvalidFeed)?;
        if (raw_end.len() == 8) != all_day {
            return Err(Problem::InvalidFeed);
        }
        let elapsed = if all_day {
            let a = NaiveDate::parse_from_str(raw, "%Y%m%d").map_err(|_| Problem::InvalidFeed)?;
            let b =
                NaiveDate::parse_from_str(raw_end, "%Y%m%d").map_err(|_| Problem::InvalidFeed)?;
            b - a
        } else {
            datetime(end, raw_end, fallback)? - start
        };
        (elapsed, 0)
    } else if let Some(d) = value(p, "DURATION") {
        duration(d)?
    } else {
        (chrono::Duration::days(i64::from(all_day)), 0)
    };
    if span < chrono::Duration::zero()
        || span > chrono::Duration::days(366)
        || (all_day && span.num_seconds() % 86400 != 0)
    {
        return Err(Problem::InvalidFeed);
    }
    let description = unescape(value(p, "DESCRIPTION").unwrap_or_default(), 8000);
    let location = unescape(value(p, "LOCATION").unwrap_or_default(), 1000);
    let meeting_url = google::meeting_link(
        &serde_json::json!({"description": description, "location": location}),
    );
    let title = unescape(
        value(p, "SUMMARY")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Untitled event"),
        1000,
    );
    Ok((
        start,
        span,
        nominal_days,
        CalendarEvent {
            title,
            all_day,
            url: value(p, "URL")
                .and_then(google::safe_link)
                .unwrap_or_default(),
            meeting_url,
            description,
            location,
            calendar_name: choice.name.clone(),
            calendar_color: choice.color.clone(),
            ..Default::default()
        },
    ))
}
fn instance(
    mut e: CalendarEvent,
    start: DateTime<Tz>,
    span: chrono::Duration,
    nominal_days: u64,
) -> Result<CalendarEvent, Problem> {
    if e.all_day {
        let end = start
            .date_naive()
            .checked_add_signed(span)
            .ok_or(Problem::InvalidFeed)?;
        e.start = start.date_naive().to_string();
        e.end = end.to_string();
    } else {
        e.start = start.to_rfc3339();
        // RFC 5545 distinguishes a calendar day (P1D) from 24 hours (PT24H).
        e.end = start
            .checked_add_days(chrono::Days::new(nominal_days))
            .ok_or(Problem::InvalidFeed)?
            .checked_add_signed(span - chrono::Duration::days(nominal_days as i64))
            .ok_or(Problem::InvalidFeed)?
            .to_rfc3339();
    }
    Ok(e)
}
fn overlaps(e: &CalendarEvent, first: NaiveDate, last: NaiveDate) -> bool {
    let mut day = first;
    while day < last {
        if e.on_date(day) {
            return true;
        }
        let Some(next) = day.succ_opt() else { break };
        day = next;
    }
    false
}
pub(super) fn parse(
    bytes: &[u8],
    month: &str,
    choice: &CalendarChoice,
) -> Result<Vec<CalendarEvent>, Problem> {
    if bytes.len() > MAX_BYTES {
        return Err(Problem::FeedLimit);
    }
    let (from, to) = google::window(month)?;
    let from = DateTime::parse_from_rfc3339(&from).map_err(|_| Problem::InvalidFeed)?;
    let to = DateTime::parse_from_rfc3339(&to).map_err(|_| Problem::InvalidFeed)?;
    let started = std::time::Instant::now();
    let mut parser = ical::IcalParser::new(Cursor::new(bytes));
    let calendar = parser
        .next()
        .ok_or(Problem::InvalidFeed)?
        .map_err(|_| Problem::InvalidFeed)?;
    if parser.next().is_some() || calendar.events.len() > 5000 {
        return Err(Problem::FeedLimit);
    }
    let zone = value(&calendar.properties, "X-WR-TIMEZONE");
    // One highest-sequence record per UID/recurrence identity. A cancelled
    // override suppresses its original occurrence even without DTSTART.
    let mut records: BTreeMap<(String, Option<i64>), &IcalEvent> = BTreeMap::new();
    for e in &calendar.events {
        let uid = value(&e.properties, "UID").ok_or(Problem::InvalidFeed)?;
        if uid.is_empty() || uid.len() > 1024 {
            return Err(Problem::FeedLimit);
        }
        let uid = uid.to_string();
        let recurrence = if let Some(p) = prop(&e.properties, "RECURRENCE-ID") {
            if param(p, "RANGE").is_some() {
                return Err(Problem::FeedLimit);
            }
            Some(datetime(p, p.value.as_deref().ok_or(Problem::InvalidFeed)?, zone)?.timestamp())
        } else {
            None
        };
        let key = (uid, recurrence);
        let rank = |e: &IcalEvent| {
            (
                value(&e.properties, "SEQUENCE")
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0),
                value(&e.properties, "DTSTAMP")
                    .unwrap_or_default()
                    .to_owned(),
            )
        };
        if records.get(&key).is_none_or(|old| rank(e) >= rank(old)) {
            records.insert(key, e);
        }
    }
    let mut out = vec![];
    let mut text_bytes = 0usize;
    for ((uid, rid), e) in &records {
        if started.elapsed() > Duration::from_secs(4) {
            return Err(Problem::FeedLimit);
        }
        if value(&e.properties, "STATUS") == Some("CANCELLED") {
            continue;
        }
        let (start, span, nominal_days, template) = event(e, zone, choice)?;
        let mut dates = vec![start];
        if rid.is_none() {
            let mut set = RRuleSet::new(start).rdate(start);
            let mut recurring = false;
            for p in &e.properties {
                if p.name == "RRULE" {
                    let rule: rrule::RRule<rrule::Unvalidated> = p
                        .value
                        .as_deref()
                        .ok_or(Problem::InvalidFeed)?
                        .parse()
                        .map_err(|_| Problem::InvalidFeed)?;
                    set = set.rrule(rule.validate(start).map_err(|_| Problem::FeedLimit)?);
                    recurring = true;
                } else if p.name == "RDATE" || p.name == "EXDATE" {
                    for raw in p.value.as_deref().ok_or(Problem::InvalidFeed)?.split(',') {
                        let date = datetime(p, raw, zone)?;
                        set = if p.name == "RDATE" {
                            set.rdate(date)
                        } else {
                            set.exdate(date)
                        };
                    }
                    recurring = true;
                } else if p.name == "EXRULE" {
                    return Err(Problem::FeedLimit);
                }
            }
            if recurring {
                let result = set
                    .after((from - span - chrono::Duration::days(1)).with_timezone(&Tz::UTC))
                    .before(to.with_timezone(&Tz::UTC))
                    .all(4096);
                if result.limited {
                    return Err(Problem::FeedLimit);
                }
                dates = result.dates;
            }
        }
        let mut seen = BTreeSet::new();
        for date in dates {
            if !seen.insert(date.timestamp())
                || (rid.is_none() && records.contains_key(&(uid.clone(), Some(date.timestamp()))))
            {
                continue;
            }
            let item = instance(template.clone(), date, span, nominal_days)?;
            if overlaps(&item, from.date_naive(), to.date_naive()) {
                text_bytes = text_bytes.saturating_add(event_text_bytes(&item));
                if text_bytes > EVENT_TEXT_BUDGET {
                    return Err(Problem::FeedLimit);
                }
                out.push(item);
            }
            if out.len() > MAX_EVENTS {
                return Err(Problem::FeedLimit);
            }
        }
    }
    out.sort_by(CalendarEvent::chronological_order);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn choice() -> CalendarChoice {
        CalendarChoice {
            id: "fixture".into(),
            name: "Work".into(),
            primary: false,
            color: "#123abc".into(),
        }
    }
    fn feed(body: &str) -> Vec<u8> {
        format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n{body}\r\nEND:VCALENDAR\r\n").into_bytes()
    }
    fn parse_events(body: &str) -> Vec<CalendarEvent> {
        parse(&feed(body), "2026-10", &choice()).unwrap()
    }
    #[test]
    fn urls_are_private_https_credentials() {
        assert_eq!(
            url("webcal://example.com/private.ics").unwrap().as_str(),
            "https://example.com/private.ics"
        );
        for invalid in [
            "http://example.com/a.ics",
            "file:///etc/passwd",
            "https://user:pass@example.com/x",
            "https://example.com/x#secret",
            "javascript:alert(1)",
        ] {
            assert_eq!(url(invalid).unwrap_err(), Problem::FeedUrl);
        }
        assert!(!format!("{:?}", SecretText("secret-feed-url".into())).contains("secret-feed-url"));
    }
    #[test]
    fn folded_text_all_day_and_meeting_links() {
        let events = parse_events(
            "BEGIN:VEVENT\r\nUID:all-day\r\nDTSTART;VALUE=DATE:20261020\r\nDTEND;VALUE=DATE:20261022\r\nSUMMARY:Trip\\, with\r\n  friends\r\nDESCRIPTION:Line one\\nLine two\r\nLOCATION:https://meet.google.com/abc-defg-hij\r\nEND:VEVENT",
        );
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.title, "Trip, with friends");
        assert_eq!(event.start, "2026-10-20");
        assert_eq!(event.end, "2026-10-22");
        assert!(event.on_date(NaiveDate::from_ymd_opt(2026, 10, 21).unwrap()));
        assert!(!event.on_date(NaiveDate::from_ymd_opt(2026, 10, 22).unwrap()));
        assert_eq!(event.description, "Line one\nLine two");
        assert!(event.join_url().is_some());
        assert_eq!(event.calendar_name, "Work");
    }
    #[test]
    fn recurrence_observes_dst_exclusions_moves_and_cancellations() {
        let events = parse_events(
            "BEGIN:VEVENT\r\nUID:weekly\r\nDTSTART;TZID=Europe/Zagreb:20261018T090000\r\nDTEND;TZID=Europe/Zagreb:20261018T100000\r\nRRULE:FREQ=WEEKLY;COUNT=4\r\nEXDATE;TZID=Europe/Zagreb:20261108T090000\r\nSUMMARY:Weekly\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:weekly\r\nRECURRENCE-ID;TZID=Europe/Zagreb:20261101T090000\r\nDTSTART;TZID=Europe/Zagreb:20261102T110000\r\nDTEND;TZID=Europe/Zagreb:20261102T120000\r\nSUMMARY:Moved\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:weekly\r\nRECURRENCE-ID;TZID=Europe/Zagreb:20261018T090000\r\nSTATUS:CANCELLED\r\nEND:VEVENT",
        );
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].start, "2026-10-25T09:00:00+01:00");
        assert_eq!(events[1].title, "Moved");
        assert_eq!(events[1].start, "2026-11-02T11:00:00+01:00");
    }
    #[test]
    fn highest_sequence_wins_and_unrelated_month_is_omitted() {
        let events = parse_events(
            "BEGIN:VEVENT\r\nUID:a\r\nSEQUENCE:2\r\nDTSTART:20261020T100000Z\r\nDURATION:PT30M\r\nSUMMARY:New\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:a\r\nSEQUENCE:1\r\nDTSTART:20261020T090000Z\r\nSUMMARY:Old\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nDTSTART:20250101T090000Z\r\nSUMMARY:Unrelated\r\nEND:VEVENT",
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "New");
        assert_eq!(events[0].end, "2026-10-20T10:30:00+00:00");
    }
    #[test]
    fn invalid_feeds_do_not_become_empty_successes() {
        assert!(parse(b"<html>Sign in</html>", "2026-10", &choice()).is_err());
        assert!(parse(&vec![b'x'; MAX_BYTES + 1], "2026-10", &choice()).is_err());
        let unknown = feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART;TZID=Custom/Unknown:20261020T100000\r\nEND:VEVENT",
        );
        assert_eq!(
            parse(&unknown, "2026-10", &choice()).unwrap_err(),
            Problem::FeedTimezone
        );
        let too_many = feed(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART:20261020T100000Z\r\nRRULE:FREQ=SECONDLY\r\nEND:VEVENT",
        );
        assert_eq!(
            parse(&too_many, "2026-10", &choice()).unwrap_err(),
            Problem::FeedLimit
        );
    }
    #[test]
    fn recurrence_cannot_multiply_large_descriptions_without_bound() {
        let body = format!(
            "BEGIN:VEVENT\r\nUID:large\r\nDTSTART:20261020T100000Z\r\nRRULE:FREQ=MINUTELY;COUNT=2000\r\nDESCRIPTION:{}\r\nEND:VEVENT",
            "x".repeat(8000)
        );
        assert_eq!(
            parse(&feed(&body), "2026-10", &choice()).unwrap_err(),
            Problem::FeedLimit
        );
    }
    #[test]
    fn nominal_days_and_elapsed_hours_differ_across_dst() {
        for (duration, expected) in [
            ("P1D", "2026-10-25T12:00:00+01:00"),
            ("PT24H", "2026-10-25T11:00:00+01:00"),
        ] {
            let body = format!(
                "BEGIN:VEVENT\r\nUID:span\r\nDTSTART;TZID=Europe/Zagreb:20261024T120000\r\nDURATION:{duration}\r\nEND:VEVENT"
            );
            assert_eq!(parse_events(&body)[0].end, expected);
        }
    }
    #[test]
    fn until_and_rdate_keep_inclusive_last_day_without_duplicates() {
        let events = parse_events(
            "BEGIN:VEVENT\r\nUID:until\r\nDTSTART;VALUE=DATE:20261024\r\nRRULE:FREQ=DAILY;UNTIL=20261026\r\nRDATE;VALUE=DATE:20261026,20261028\r\nEND:VEVENT",
        );
        assert_eq!(
            events
                .iter()
                .map(|event| event.start.as_str())
                .collect::<Vec<_>>(),
            ["2026-10-24", "2026-10-25", "2026-10-26", "2026-10-28"]
        );
    }
    #[test]
    fn all_day_recurrence_retains_dates_across_dst() {
        let events = parse_events(
            "BEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:20261024\r\nDURATION:P2D\r\nRRULE:FREQ=DAILY;COUNT=3\r\nEND:VEVENT",
        );
        assert_eq!(events.len(), 3);
        assert_eq!(events[1].start, "2026-10-25");
        assert_eq!(events[1].end, "2026-10-27");
    }
}

#[cfg(test)]
#[tokio::test]
#[ignore = "requires a disposable Secret Service on a private D-Bus session"]
async fn private_subscription_commit_remove_and_stale_replacement() {
    crate::calendar::tests::require_private_keyring();
    keyring::ready().await.unwrap();
    let feeds = Feeds {
        feeds: ["Work", "Personal"]
            .into_iter()
            .map(|name| Feed {
                choice: CalendarChoice {
                    id: name.into(),
                    name: name.into(),
                    primary: false,
                    color: String::new(),
                },
                url: SecretText(format!(
                    "https://example.invalid/{name}/private-feed-token.ics"
                )),
            })
            .collect(),
    };
    let connection = Connection {
        backend: Backend::Subscription,
        account: "Calendar subscriptions".into(),
        calendar_id: "Work".into(),
        calendar_name: "Work".into(),
        revision: store::nonce().unwrap(),
        credential: store::nonce().unwrap(),
        selections: feeds.feeds.iter().map(|f| f.choice.clone()).collect(),
        subscriptions: feeds.feeds.iter().map(|f| f.choice.clone()).collect(),
    };
    let expected = store::load_connection();
    let connected = commit(&expected, connection, &feeds).await.unwrap();
    let secret: Feeds = keyring::read(&connected.credential, false).await.unwrap();
    assert_eq!(secret.feeds[1].url.0, feeds.feeds[1].url.0);
    let metadata = std::fs::read_to_string(store::account_path().unwrap()).unwrap();
    assert!(!metadata.contains("private-feed-token") && !metadata.contains("example.invalid"));
    let candidate = Connection {
        credential: store::nonce().unwrap(),
        revision: store::nonce().unwrap(),
        ..connected.clone()
    };
    assert_eq!(
        commit(&expected, candidate.clone(), &feeds).await,
        Err(Problem::Changed)
    );
    assert!(
        keyring::read::<Feeds>(&candidate.credential, false)
            .await
            .is_err()
    );
    assert!(
        keyring::read::<Feeds>(&connected.credential, false)
            .await
            .is_ok()
    );
    let remaining = remove_subscription(connected.clone(), "Work".into())
        .await
        .unwrap();
    assert_eq!(remaining.selected_calendars()[0].name, "Personal");
    assert!(
        keyring::read::<Feeds>(&connected.credential, false)
            .await
            .is_err()
    );
    let remaining_feeds: Feeds = keyring::read(&remaining.credential, false).await.unwrap();
    assert_eq!(remaining_feeds.feeds.len(), 1);
    let removed = remove_subscription(remaining.clone(), "Personal".into())
        .await
        .unwrap();
    assert!(!removed.connected() && !removed.cleanup_pending());
    assert!(
        keyring::read::<Feeds>(&remaining.credential, false)
            .await
            .is_err()
    );
}
