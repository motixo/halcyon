use super::*;
use std::os::unix::fs::PermissionsExt;
fn fixture() -> PathBuf {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).unwrap();
    let p = std::env::temp_dir().join(format!("halcyon-usage-{:x}", u64::from_le_bytes(bytes)));
    std::fs::create_dir_all(&p).unwrap();
    p.join("usage.json")
}
#[test]
fn first_report_persists_a_random_private_identity_and_reserves_retry() {
    let path = fixture();
    let token = claim(&path, "2026-10-23", 1000)
        .unwrap()
        .expect("first daily report");
    assert_eq!(token.len(), 64);
    assert!(token.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(claim(&path, "2026-10-23", 1001).unwrap().is_none());
    assert_eq!(claim(&path, "2026-10-23", 22600).unwrap(), Some(token));
}
#[test]
fn success_survives_restart_and_next_day_keeps_the_same_identity() {
    let path = fixture();
    let token = claim(&path, "2026-10-23", 1000).unwrap().unwrap();
    complete(&path, &token, "2026-10-23").unwrap();
    assert!(claim(&path, "2026-10-23", 24000).unwrap().is_none());
    assert_eq!(claim(&path, "2026-10-24", 90000).unwrap(), Some(token));
}
#[test]
fn concurrent_claims_only_schedule_one_request() {
    let path = fixture();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let path = path.clone();
            let b = barrier.clone();
            std::thread::spawn(move || {
                b.wait();
                claim(&path, "2026-10-23", 1000).ok().flatten()
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .filter_map(|t| t.join().unwrap())
            .count(),
        1
    );
}
#[test]
fn corrupted_state_does_not_silently_register_another_installation() {
    let path = fixture();
    std::fs::write(&path, "broken").unwrap();
    assert!(claim(&path, "2026-10-23", 1000).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "broken");
}
#[test]
fn late_response_cannot_replace_a_new_identity() {
    let path = fixture();
    let token = claim(&path, "2026-10-23", 1000).unwrap().unwrap();
    complete(&path, &"a".repeat(64), "2026-10-23").unwrap();
    let state: State = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(state.installation, token);
    assert!(state.last_success.is_none());
}
#[test]
fn endpoint_is_limited_to_a_production_google_deployment() {
    assert!(endpoint("https://script.google.com/macros/s/ABC_123/exec").is_some());
    for value in [
        "",
        "https://script.google.com/macros/s//exec",
        "https://script.google.com/macros/s/%20/exec",
        "http://script.google.com/macros/s/ABC/exec",
        "https://evil.test/macros/s/ABC/exec",
        "https://user:secret@script.google.com/macros/s/ABC/exec",
        "https://script.google.com/macros/s/ABC/dev",
        "https://script.google.com/macros/s/ABC/exec?account=private",
        "https://script.google.com:123/macros/s/ABC/exec",
    ] {
        assert!(endpoint(value).is_none(), "{value}");
    }
}
#[test]
fn report_contains_only_the_documented_fields() {
    let report = serde_json::to_value(Report {
        schema: 2,
        installation: &"a".repeat(64),
        version: "0.2.0",
        country: Some("HR"),
    })
    .unwrap();
    assert_eq!(report.as_object().unwrap().len(), 4);
    assert_eq!(report["schema"], 2);
    assert_eq!(report["version"], "0.2.0");
    assert_eq!(report["country"], "HR");
    assert!(report.get("installation").is_some());
    assert!(
        !available(),
        "test builds must never start reporting automatically"
    );
}
async fn reply(response: &'static str) -> Result<String, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = [0u8; 4096];
        let _ = stream.read(&mut request);
        stream.write_all(response.as_bytes()).unwrap();
    });
    let result = send(
        &client().unwrap(),
        reqwest::Url::parse(&format!("http://{address}/")).unwrap(),
        &"a".repeat(64),
        None,
    )
    .await;
    server.join().unwrap();
    result
}
#[tokio::test]
async fn only_an_explicit_success_with_a_valid_day_completes_a_report() {
    assert_eq!(reply("HTTP/1.1 200 OK\r\nContent-Length: 30\r\nConnection: close\r\n\r\n{\"ok\":true,\"day\":\"2026-10-23\"}").await.unwrap(),"2026-10-23");
    assert!(
        reply("HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\n{\"ok\":false}")
            .await
            .is_err()
    );
    assert!(
        reply("HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n<html")
            .await
            .is_err()
    );
    assert!(reply("HTTP/1.1 302 Found\r\nLocation: https://example.com/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.is_err());
}

#[test]
fn country_state_round_trips_without_replacing_the_existing_identity() {
    let path = fixture();
    let id = claim(&path, "2026-10-23", 1000).unwrap().unwrap();
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    json["country"] = serde_json::json!("HR");
    std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
    let state = load(&path).expect("country-compatible state");
    assert_eq!(state.installation, id);
    assert_eq!(serde_json::to_value(state).unwrap()["country"], "HR");
}

#[test]
fn legacy_state_migrates_and_a_late_lookup_cannot_change_identity_or_first_country() {
    let path = fixture();
    let id = "b".repeat(64);
    std::fs::write(
        &path,
        format!(r#"{{"installation":"{id}","last_success":null,"retry_after":0}}"#),
    )
    .unwrap();
    assert_eq!(
        claim(&path, "2026-10-23", 1000).unwrap().as_deref(),
        Some(id.as_str())
    );
    assert_eq!(
        remember_country(&path, &id, "HR").unwrap().as_deref(),
        Some("HR")
    );
    assert_eq!(
        remember_country(&path, &id, "US").unwrap().as_deref(),
        Some("HR")
    );
    assert_eq!(
        remember_country(&path, &"c".repeat(64), "DE").unwrap(),
        None
    );
    assert_eq!(load(&path).unwrap().country.as_deref(), Some("HR"));
}

// A local HTTP boundary exercises the real client, without sending activity or IPs online.
fn local_server(status: &str, body: &str) -> (String, std::thread::JoinHandle<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let task = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(8);
        let (mut stream, _) = loop {
            if let Ok(stream) = listener.accept() {
                break stream;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "local client never connected"
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut request = Vec::new();
        loop {
            let mut buffer = [0; 4096];
            let n = stream.read(&mut buffer).unwrap();
            assert!(n > 0, "incomplete request");
            request.extend_from_slice(&buffer[..n]);
            let text = String::from_utf8_lossy(&request);
            if let Some(end) = text.find("\r\n\r\n") {
                let length: usize = text[..end]
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap())
                    })
                    .unwrap_or(0);
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(request).unwrap()
    });
    (url, task)
}

#[tokio::test]
async fn country_lookup_discards_ip_caches_first_country_and_never_sends_identity() {
    let path = fixture();
    let id = claim(&path, "2026-10-23", 1000).unwrap().unwrap();
    let (url, request) = local_server("200 OK", r#"{"ip":"192.0.2.1","country":"HR"}"#);
    assert_eq!(country_for(&path, &id, &url).await.as_deref(), Some("HR"));
    let request = request.join().unwrap();
    assert!(request.starts_with("GET / HTTP/1.1"));
    assert!(!request.contains(&id));
    assert!(
        !std::fs::read_to_string(&path)
            .unwrap()
            .contains("192.0.2.1")
    );
    // The server has gone away; this must use the persisted cache after a restart.
    assert_eq!(country_for(&path, &id, &url).await.as_deref(), Some("HR"));
}

#[tokio::test]
async fn failed_country_lookup_still_sends_activity_with_unknown_country() {
    let path = fixture();
    let id = claim(&path, "2026-10-23", 1000).unwrap().unwrap();
    let (country_url, country_request) = local_server("503 Unavailable", "unavailable");
    let (report_url, report_request) = local_server("200 OK", r#"{"ok":true,"day":"2026-10-23"}"#);
    assert_eq!(
        report_once(
            &path,
            &client().unwrap(),
            reqwest::Url::parse(&report_url).unwrap(),
            &id,
            &country_url
        )
        .await
        .unwrap(),
        "2026-10-23"
    );
    country_request.join().unwrap();
    let request = report_request.join().unwrap();
    let report: serde_json::Value =
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(report.as_object().unwrap().len(), 4);
    assert_eq!(report["schema"], 2);
    assert_eq!(report["installation"], id);
    assert!(report["country"].is_null());
    assert!(load(&path).unwrap().country.is_none());
    // Failure was not cached: a later report can acquire a country.
    let (url, request) = local_server("200 OK", r#"{"country":"DE"}"#);
    assert_eq!(country_for(&path, &id, &url).await.as_deref(), Some("DE"));
    request.join().unwrap();
}

#[tokio::test]
async fn invalid_or_oversized_country_responses_never_enter_state() {
    for body in [
        r#"{"country":"hr"}"#.to_string(),
        r#"{"country":null}"#.to_string(),
        "<html>".into(),
        format!(r#"{{"country":"US","extra":"{}"}}"#, "x".repeat(1024)),
    ] {
        let path = fixture();
        let id = claim(&path, "2026-10-23", 1000).unwrap().unwrap();
        let (url, request) = local_server("200 OK", &body);
        assert!(country_for(&path, &id, &url).await.is_none());
        request.join().unwrap();
        assert!(load(&path).unwrap().country.is_none());
    }
}
