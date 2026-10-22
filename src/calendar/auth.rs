use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::watch,
};

#[derive(Clone)]
pub struct Authorization(Arc<Pending>);
struct Pending {
    listener: TcpListener,
    client: GoogleClient,
    expected: Connection,
    state: String,
    verifier: String,
    redirect: String,
    url: String,
    cancelled: watch::Sender<bool>,
}
impl std::fmt::Debug for Authorization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Authorization([redacted])")
    }
}
impl Authorization {
    pub fn url(&self) -> &str {
        &self.0.url
    }
    pub fn cancel(&self) {
        self.0.cancelled.send_replace(true);
    }
    fn cancelled(&self) -> bool {
        *self.0.cancelled.borrow()
    }
}
pub(super) fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
pub(super) fn authorization_url(
    client: &GoogleClient,
    redirect: &str,
    state: &str,
    verifier: &str,
) -> String {
    let mut url = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
    url.query_pairs_mut().extend_pairs([
        ("client_id", client.client_id.as_str()),
        ("redirect_uri", redirect),
        ("response_type", "code"),
        ("scope", google::SCOPE),
        ("access_type", "offline"),
        ("prompt", "consent select_account"),
        ("state", state),
        ("code_challenge", challenge(verifier).as_str()),
        ("code_challenge_method", "S256"),
    ]);
    url.into()
}
pub async fn begin_sign_in() -> Result<Authorization, Problem> {
    if store::preview() {
        return Err(Problem::Preview);
    }
    let client = store::load_client()?;
    let expected = store::load_connection();
    if expected.cleanup_pending() {
        return Err(Problem::Cleanup);
    }
    tokio::time::timeout(Duration::from_secs(90), keyring::ready())
        .await
        .map_err(|_| Problem::Locked)??;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|_| Problem::Network)?;
    let redirect = format!(
        "http://127.0.0.1:{}/",
        listener.local_addr().map_err(|_| Problem::Network)?.port()
    );
    let state = store::nonce()?;
    let verifier = store::nonce()?;
    let url = authorization_url(&client, &redirect, &state, &verifier);
    Ok(Authorization(Arc::new(Pending {
        listener,
        client,
        expected,
        state,
        verifier,
        redirect,
        url,
        cancelled: watch::channel(false).0,
    })))
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Callback {
    Ignore,
    Denied,
    Code(String),
}
pub(super) fn callback(target: &str, state: &str) -> Callback {
    if !target.starts_with("/?") {
        return Callback::Ignore;
    }
    let Ok(url) = reqwest::Url::parse(&format!("http://127.0.0.1{target}")) else {
        return Callback::Ignore;
    };
    let pairs: Vec<_> = url.query_pairs().collect();
    let states: Vec<_> = pairs.iter().filter(|(key, _)| key == "state").collect();
    if states.len() != 1 || states[0].1 != state {
        return Callback::Ignore;
    }
    if pairs.iter().any(|(key, _)| key == "error") {
        return Callback::Denied;
    }
    let codes: Vec<_> = pairs.iter().filter(|(key, _)| key == "code").collect();
    if codes.len() == 1 && !codes[0].1.is_empty() && codes[0].1.len() <= 4096 {
        Callback::Code(codes[0].1.to_string())
    } else {
        Callback::Ignore
    }
}
async fn receive(auth: &Authorization) -> Result<String, Problem> {
    let mut cancelled = auth.0.cancelled.subscribe();
    loop {
        if auth.cancelled() {
            return Err(Problem::Cancelled);
        }
        let accepted = tokio::select! {
            _ = cancelled.changed() => return Err(Problem::Cancelled),
            accepted = auth.0.listener.accept() => accepted.map_err(|_| Problem::Network)?,
        };
        let (mut socket, peer) = accepted;
        if !peer.ip().is_loopback() {
            continue;
        }
        let request = tokio::time::timeout(Duration::from_secs(3), async {
            let mut bytes = Vec::new();
            let mut part = [0; 1024];
            while bytes.len() < 16384 {
                let count = socket.read(&mut part).await.map_err(|_| Problem::Network)?;
                if count == 0 {
                    break;
                }
                bytes.extend_from_slice(&part[..count]);
                if bytes.windows(4).any(|value| value == b"\r\n\r\n") {
                    break;
                }
            }
            Ok::<_, Problem>(bytes)
        })
        .await;
        let result = match request {
            Ok(Ok(bytes)) if bytes.len() < 16384 => {
                let request = String::from_utf8_lossy(&bytes);
                let mut line = request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .split_whitespace();
                if line.next() == Some("GET") {
                    callback(line.next().unwrap_or_default(), &auth.0.state)
                } else {
                    Callback::Ignore
                }
            }
            _ => Callback::Ignore,
        };
        let (status, body) = match result {
            Callback::Ignore => (
                "400 Bad Request",
                "This sign-in request is no longer valid. Return to Desktop Cards and try again.",
            ),
            Callback::Denied => (
                "200 OK",
                "Sign-in was not completed. You can return to Desktop Cards.",
            ),
            Callback::Code(_) => (
                "200 OK",
                "Google sign-in received. Return to Desktop Cards to see your connection status.",
            ),
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            socket.write_all(response.as_bytes()),
        )
        .await;
        match result {
            Callback::Code(code) => return Ok(code),
            Callback::Denied => return Err(Problem::Denied),
            Callback::Ignore => {}
        }
    }
}
pub async fn finish_sign_in(auth: Authorization) -> Result<Connection, Problem> {
    let mut cancelled = auth.0.cancelled.subscribe();
    let prepared = tokio::select! {
        _ = cancelled.changed() => return Err(Problem::Cancelled),
        result = tokio::time::timeout(Duration::from_secs(300), prepare(&auth)) => result.map_err(|_| Problem::TimedOut)??,
    };
    commit(&auth, prepared.0, prepared.1).await
}
async fn prepare(auth: &Authorization) -> Result<(Connection, Credential), Problem> {
    if auth.cancelled() {
        return Err(Problem::Cancelled);
    }
    let code = receive(auth).await?;
    let client = google::http()?;
    let token = google::json(
        client
            .post(google::TOKEN)
            .form(&[
                ("client_id", auth.0.client.client_id.as_str()),
                ("client_secret", auth.0.client.client_secret.as_str()),
                ("code", &code),
                ("redirect_uri", &auth.0.redirect),
                ("grant_type", "authorization_code"),
                ("code_verifier", &auth.0.verifier),
            ])
            .send()
            .await,
    )
    .await?;
    if let Some(scopes) = token["scope"].as_str() {
        if [
            "https://www.googleapis.com/auth/calendar.events.readonly",
            "https://www.googleapis.com/auth/calendar.calendarlist.readonly",
        ]
        .iter()
        .any(|required| !scopes.split_whitespace().any(|scope| scope == *required))
        {
            return Err(Problem::Denied);
        }
    }
    let access = token["access_token"]
        .as_str()
        .ok_or(Problem::InvalidResponse)?;
    let refresh = token["refresh_token"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or(Problem::SignIn)?;
    let user = google::json(
        client
            .get("https://openidconnect.googleapis.com/v1/userinfo")
            .bearer_auth(access)
            .send()
            .await,
    )
    .await?;
    let account = user["email"]
        .as_str()
        .filter(|value| value.contains('@') && value.len() <= 320)
        .ok_or(Problem::Denied)?;
    if user["email_verified"] != true || !user["sub"].is_string() {
        return Err(Problem::Denied);
    }
    let choices = google::calendars(access).await?;
    let choice = choices
        .iter()
        .find(|choice| choice.primary)
        .or_else(|| choices.first())
        .ok_or(Problem::Denied)?;
    if auth.cancelled() {
        return Err(Problem::Cancelled);
    }
    let connection = Connection {
        backend: Backend::Google,
        account: account.into(),
        calendar_id: choice.id.clone(),
        calendar_name: choice.name.clone(),
        revision: store::nonce()?,
        credential: store::nonce()?,
    };
    Ok((
        connection,
        Credential {
            refresh_token: refresh.into(),
            client: auth.0.client.clone(),
        },
    ))
}
/// Removes a credential that is no longer used; if the keyring refuses,
/// remembers its ID so a later sign-in or disconnect removes it.
async fn forget(id: &str) {
    if id.is_empty() || keyring::remove(id).await.is_ok() {
        return;
    }
    let mut stale = store::stale_credentials();
    if !stale.iter().any(|known| known == id) {
        stale.push(id.to_string());
        store::set_stale_credentials(&stale);
    }
}

async fn commit(
    auth: &Authorization,
    connection: Connection,
    credential: Credential,
) -> Result<Connection, Problem> {
    if auth.cancelled() {
        return Err(Problem::Cancelled);
    }
    keyring::save(&connection.credential, &credential).await?;
    let saved = if auth.cancelled() {
        Err(Problem::Cancelled)
    } else {
        store::replace(&auth.0.expected, &connection)
    };
    if let Err(problem) = saved {
        forget(&connection.credential).await;
        return Err(problem);
    }
    if auth.0.expected.backend == Backend::Google {
        forget(&auth.0.expected.credential).await;
    }
    // Credentials left behind by an earlier switch get another try now that
    // the keyring is open.
    super::retry_stale_credentials().await;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn pending() -> Authorization {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let redirect = format!("http://{}/", listener.local_addr().unwrap());
        Authorization(Arc::new(Pending {
            listener,
            redirect,
            client: GoogleClient {
                client_id: "fixture.apps.googleusercontent.com".into(),
                client_secret: "fixture".into(),
            },
            expected: Connection::default(),
            state: "expected".into(),
            verifier: "unused".into(),
            url: String::new(),
            cancelled: watch::channel(false).0,
        }))
    }
    #[tokio::test]
    async fn loopback_ignores_unrelated_requests_then_accepts_matching_callback() {
        let auth = pending().await;
        let waiting = tokio::spawn({
            let auth = auth.clone();
            async move { receive(&auth).await }
        });
        let client = google::http().unwrap();
        let response = client
            .get(format!("{}?code=wrong&state=other", auth.0.redirect))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
        assert!(!response.text().await.unwrap().contains("wrong"));
        let response = client
            .get(format!(
                "{}?code=fixture-code&state=expected",
                auth.0.redirect
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert!(!response.text().await.unwrap().contains("fixture-code"));
        assert_eq!(waiting.await.unwrap(), Ok("fixture-code".into()));
    }
    #[tokio::test]
    async fn cancelling_waiting_sign_in_finishes_without_account_changes() {
        let auth = pending().await;
        let waiting = tokio::spawn({
            let auth = auth.clone();
            async move { finish_sign_in(auth).await }
        });
        tokio::task::yield_now().await;
        auth.cancel();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), waiting)
                .await
                .unwrap()
                .unwrap(),
            Err(Problem::Cancelled)
        );
    }
    #[tokio::test]
    async fn browser_denial_returns_actionable_error_without_token_exchange() {
        let auth = pending().await;
        let waiting = tokio::spawn({
            let auth = auth.clone();
            async move { finish_sign_in(auth).await }
        });
        let response = google::http()
            .unwrap()
            .get(format!(
                "{}?error=access_denied&state=expected",
                auth.0.redirect
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(waiting.await.unwrap(), Err(Problem::Denied));
    }
}
