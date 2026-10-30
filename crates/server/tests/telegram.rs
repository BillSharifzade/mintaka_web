//! The Telegram bot (docs/CONTRACTS.md §19) against a Bot API double in this process: the form's
//! link only while the bot runs, `/start` with good, bad and decoy codes, the relay both ways,
//! `/stop`, a restart that keeps the links, the brief's language, and the per-chat limits.

mod common;

use std::collections::HashSet;
use std::convert::Infallible;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use common::{BROWSER, Client, TestServer, config, request};
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use server::TelegramSettings;
use tokio::net::TcpListener;

const TOKEN: &str = "123456:test-token_x";
const TEAM: i64 = -1_001_234_567_890;
const BOT: &str = "mintaka_test_bot";
/// A client who blocked the bot: every message to this chat is refused.
const BLOCKED: i64 = 900;

/// One call the server made: the method, its JSON body and the message number the double gave.
#[derive(Clone)]
struct Call {
    method: String,
    body: Value,
    id: i64,
}

#[derive(Default)]
struct State {
    queue: Vec<Value>,
    next_update: i64,
    next_message: i64,
    calls: Vec<Call>,
    offsets: Vec<i64>,
}

/// The Bot API, as much of it as the server uses.
struct Telegram {
    base: String,
    state: Mutex<State>,
}

impl Telegram {
    async fn start() -> Arc<Telegram> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let api = Arc::new(Telegram {
            base: format!("http://{}", listener.local_addr().unwrap()),
            state: Mutex::new(State { next_update: 1000, next_message: 5000, ..State::default() }),
        });
        let serving = Arc::clone(&api);
        tokio::spawn(async move {
            while let Ok((tcp, _)) = listener.accept().await {
                let api = Arc::clone(&serving);
                tokio::spawn(async move {
                    let service = service_fn(move |req| {
                        let api = Arc::clone(&api);
                        async move { Ok::<_, Infallible>(api.handle(req).await) }
                    });
                    let _ = hyper::server::conn::http1::Builder::new().serve_connection(TokioIo::new(tcp), service).await;
                });
            }
        });
        api
    }

    fn settings(&self, token: &str) -> TelegramSettings {
        TelegramSettings { bot_token: token.into(), chat_id: TEAM, api_base: self.base.clone() }
    }

    async fn handle(&self, req: Request<Incoming>) -> Response<Full<Bytes>> {
        let path = req.uri().path().to_owned();
        let body: Value = serde_json::from_slice(&req.into_body().collect().await.unwrap().to_bytes()).unwrap_or(Value::Null);
        let answer = |status: u16, value: Value| {
            Response::builder().status(status).header("content-type", "application/json").body(Full::new(Bytes::from(value.to_string()))).unwrap()
        };
        let Some(method) = path.strip_prefix(&format!("/bot{TOKEN}/")) else {
            return answer(401, json!({ "ok": false, "error_code": 401, "description": "Unauthorized" }));
        };
        match method {
            "getMe" => answer(200, json!({ "ok": true, "result": { "id": 1, "is_bot": true, "first_name": "Mintaka", "username": BOT } })),
            "getUpdates" => {
                let offset = body["offset"].as_i64().unwrap_or(0);
                // A short long poll: answer as soon as something is queued, or after 300 ms.
                for _ in 0..15 {
                    {
                        let mut state = self.state.lock().unwrap();
                        state.queue.retain(|u| u["update_id"].as_i64().unwrap() >= offset);
                        if !state.offsets.last().is_some_and(|o| *o == offset) {
                            state.offsets.push(offset);
                        }
                        if !state.queue.is_empty() {
                            return answer(200, json!({ "ok": true, "result": state.queue.clone() }));
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                answer(200, json!({ "ok": true, "result": [] }))
            }
            "sendMessage" | "copyMessage" => {
                let chat = body["chat_id"].as_i64().unwrap();
                let mut state = self.state.lock().unwrap();
                state.next_message += 1;
                let id = state.next_message;
                state.calls.push(Call { method: method.to_owned(), body: body.clone(), id });
                if chat == BLOCKED {
                    return answer(403, json!({ "ok": false, "error_code": 403, "description": "Forbidden: bot was blocked by the user" }));
                }
                let result = if method == "sendMessage" {
                    json!({ "message_id": id, "date": 0, "chat": { "id": chat, "type": "private" }, "text": body["text"] })
                } else {
                    json!({ "message_id": id })
                };
                answer(200, json!({ "ok": true, "result": result }))
            }
            _ => answer(404, json!({ "ok": false, "error_code": 404, "description": "Not Found" })),
        }
    }

    /// Queues an update with this message; returns its update id.
    fn push(&self, message: Value) -> i64 {
        let mut state = self.state.lock().unwrap();
        state.next_update += 1;
        let id = state.next_update;
        state.queue.push(json!({ "update_id": id, "message": message }));
        id
    }

    fn calls(&self) -> Vec<Call> {
        self.state.lock().unwrap().calls.clone()
    }

    /// What the bot sent after call `from`, leaving out the new-brief notifications.
    fn bot_calls(&self, from: usize) -> Vec<String> {
        self.calls().iter().skip(from).filter(|c| !team_text(c, "New brief")).map(|c| format!("{} {}", c.method, c.body)).collect()
    }

    /// The first call after `from` that `matches`, within 5 s.
    async fn wait(&self, from: usize, what: &str, matches: impl Fn(&Call) -> bool) -> Call {
        for _ in 0..250 {
            if let Some(call) = self.calls().into_iter().skip(from).find(&matches) {
                return call;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let seen: Vec<String> = self.calls().iter().skip(from).map(|c| format!("{} {}", c.method, c.body)).collect();
        panic!("no {what} within 5 s; calls: {seen:#?}");
    }

    /// Waits until the server has handled `update` (it asked for the updates after it).
    async fn handled(&self, update: i64) {
        for _ in 0..250 {
            if self.state.lock().unwrap().offsets.iter().any(|o| *o > update) {
                // …and finished what it does with it.
                tokio::time::sleep(Duration::from_millis(100)).await;
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("update {update} not handled within 5 s");
    }
}

fn private(chat: i64, message_id: i64, text: &str) -> Value {
    json!({ "message_id": message_id, "date": 0, "chat": { "id": chat, "type": "private" },
            "from": { "id": chat, "is_bot": false, "first_name": "Client" }, "text": text })
}

fn team_reply(message_id: i64, to: i64, text: &str) -> Value {
    json!({ "message_id": message_id, "date": 0, "chat": { "id": TEAM, "type": "supergroup" },
            "from": { "id": 7, "is_bot": false, "first_name": "Bill" }, "text": text,
            "reply_to_message": { "message_id": to, "date": 0, "chat": { "id": TEAM, "type": "supergroup" } } })
}

fn sent_to(call: &Call, method: &str, chat: i64) -> bool {
    call.method == method && call.body["chat_id"] == chat
}

/// A text the bot posted in the team chat, starting with `start` (the new-brief notification goes
/// there too, at its own time).
fn team_text(call: &Call, start: &str) -> bool {
    sent_to(call, "sendMessage", TEAM) && call.body["text"].as_str().is_some_and(|t| t.starts_with(start))
}

async fn with_bot(name: &str, api: &Telegram) -> TestServer {
    let mut cfg = config(name);
    cfg.telegram = Some(api.settings(TOKEN));
    TestServer::start(cfg).await
}

/// Sends a brief (JSON, no proof of work); returns the answer and the brief's id.
async fn brief(server: &TestServer, lang: &str, extra: Value) -> (Value, String) {
    let mut body = json!({ "name": "Bill", "telegram": "mintaka_tj", "lang": lang, "website": "" });
    body.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    let before: HashSet<_> = briefs(&server.data_dir).into_iter().collect();
    let reply = Client::h1(server.addr())
        .await
        .send(request("POST", "/api/contact", &[("content-type", "application/json"), BROWSER], Bytes::from(body.to_string())))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let id = briefs(&server.data_dir).into_iter().find(|f| !before.contains(f)).unwrap_or_default();
    (reply.json(), id)
}

/// The ids of the stored briefs (`{id}.json.age`).
fn briefs(data_dir: &Path) -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(data_dir.join("contact")) else { return vec![] };
    dir.map(|e| e.unwrap().file_name().into_string().unwrap().trim_end_matches(".json.age").to_owned()).collect()
}

/// The `start` code of a link, checked against Telegram's rules.
fn code(answer: &Value) -> String {
    let link = answer["bot_link"].as_str().unwrap_or_else(|| panic!("no bot_link in {answer}"));
    let code = link.strip_prefix(&format!("https://t.me/{BOT}?start=")).unwrap_or_else(|| panic!("{link}"));
    assert!((1..=64).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'), "{code}");
    code.to_owned()
}

/// Connects `chat` to a new brief; returns the brief's id and the team header's message number.
async fn connect(server: &TestServer, api: &Telegram, chat: i64, lang: &str) -> (String, i64) {
    let (answer, id) = brief(server, lang, json!({})).await;
    let from = api.calls().len();
    api.push(private(chat, 1, &format!("/start {}", code(&answer))));
    api.wait(from, "the answer", |c| sent_to(c, "sendMessage", chat)).await;
    let header = api.wait(from, "the team's header", |c| team_text(c, &format!("Brief {id} continues in Telegram."))).await;
    (id, header.id)
}

#[tokio::test]
async fn the_link_comes_only_from_a_running_bot() {
    // No bot configured: the answer is exactly as before.
    let plain = TestServer::start(config("tg-none")).await;
    let (answer, _) = brief(&plain, "en", json!({})).await;
    assert_eq!(answer, json!({ "ok": true }));

    // A bot whose token Telegram refuses: no link either.
    let api = Telegram::start().await;
    let mut cfg = config("tg-refused");
    cfg.telegram = Some(api.settings("999:wrong"));
    let refused = TestServer::start(cfg).await;
    let (answer, _) = brief(&refused, "en", json!({})).await;
    assert_eq!(answer, json!({ "ok": true }), "the bot isn't answering, so the page doesn't promise it");

    // A running bot: a link, and the team's notification as before.
    let server = with_bot("tg-link", &api).await;
    let from = api.calls().len();
    let (answer, id) = brief(&server, "en", json!({ "kind": ["ai"], "timeline": "asap" })).await;
    assert_eq!(answer["ok"], true);
    code(&answer);
    let note = api.wait(from, "the notification", |c| team_text(c, "New brief")).await;
    assert_eq!(note.body["text"], format!("New brief · ai · asap · {id}"));

    // The no-JS form: the usual redirect, no link in a URL.
    let form = Client::h1(server.addr())
        .await
        .send(request("POST", "/api/contact", &[("content-type", "application/x-www-form-urlencoded"), BROWSER], Bytes::from_static(b"lang=en&name=Bill&telegram=mintaka_tj")))
        .await;
    assert_eq!((form.status, form.header("location")), (StatusCode::SEE_OTHER, "/en/sent/"));

    // The honeypot: a link of the same shape, which opens nothing.
    let (decoy, _) = brief(&server, "en", json!({ "website": "http://spam.example" })).await;
    let decoy = code(&decoy);
    assert_eq!(decoy.len(), code(&answer).len());
    let from = api.calls().len();
    let update = api.push(private(301, 1, &format!("/start {decoy}")));
    api.handled(update).await;
    assert!(api.calls().iter().skip(from).all(|c| c.body["chat_id"] != 301), "a decoy gets no answer");
}

#[tokio::test]
async fn start_connects_and_the_relay_works_both_ways() {
    let api = Telegram::start().await;
    let server = with_bot("tg-relay", &api).await;
    let (answer, id) = brief(&server, "en", json!({})).await;
    let code = code(&answer);

    let from = api.calls().len();
    api.push(private(555, 1, &format!("/start {code}")));
    let welcome = api.wait(from, "the answer", |c| sent_to(c, "sendMessage", 555)).await;
    assert_eq!(welcome.body["text"], format!("Linked to brief {id}."));
    let header = api.wait(from, "the header", |c| team_text(c, &format!("Brief {id} continues in Telegram."))).await;
    assert!(header.body.get("reply_parameters").is_none());

    // The client writes: a copy in the team chat, under the header.
    let from = api.calls().len();
    api.push(private(555, 2, "Here is more about the project."));
    let copy = api.wait(from, "the copy", |c| sent_to(c, "copyMessage", TEAM)).await;
    assert_eq!((copy.body["from_chat_id"].as_i64(), copy.body["message_id"].as_i64()), (Some(555), Some(2)));
    assert_eq!(copy.body["reply_parameters"]["message_id"].as_i64(), Some(header.id));

    // The team replies to the copy: a copy to the client, as the bot's own message.
    let from = api.calls().len();
    api.push(team_reply(77, copy.id, "Thank you, we'll call tomorrow."));
    let back = api.wait(from, "the reply", |c| sent_to(c, "copyMessage", 555)).await;
    assert_eq!((back.body["from_chat_id"].as_i64(), back.body["message_id"].as_i64()), (Some(TEAM), Some(77)));
    assert!(back.body.get("reply_parameters").is_none());
    // A reply to the header reaches the client too, and so does a reply to the team's own reply.
    let from = api.calls().len();
    api.push(team_reply(78, header.id, "One more question."));
    api.wait(from, "a reply to the header", |c| sent_to(c, "copyMessage", 555) && c.body["message_id"] == 78).await;
    api.push(team_reply(79, 77, "And another."));
    api.wait(from, "a reply to a reply", |c| sent_to(c, "copyMessage", 555) && c.body["message_id"] == 79).await;
    // An admin posting anonymously appears as a bot; in the team chat that's still the team.
    let mut anonymous = team_reply(82, copy.id, "From an anonymous admin.");
    anonymous["from"] = json!({ "id": 1_087_968_824, "is_bot": true, "first_name": "Group", "username": "GroupAnonymousBot" });
    api.push(anonymous);
    api.wait(from, "an anonymous admin's reply", |c| sent_to(c, "copyMessage", 555) && c.body["message_id"] == 82).await;

    // The same code again: from this chat, a reminder; from another, refused.
    let from = api.calls().len();
    api.push(private(555, 3, &format!("/start {code}")));
    let again = api.wait(from, "the reminder", |c| sent_to(c, "sendMessage", 555)).await;
    assert_eq!(again.body["text"], format!("Already linked to brief {id}."));
    api.push(private(556, 1, &format!("/start {code}")));
    let taken = api.wait(from, "the refusal", |c| sent_to(c, "sendMessage", 556)).await;
    assert_eq!(taken.body["text"], "Taken by another chat.");

    // Ignored: a forged code, a team message that replies to nothing of ours, another group, another
    // bot, and an unconnected chat's ordinary message.
    let from = api.calls().len();
    let mut forged = code.clone().into_bytes();
    let last = forged.len() - 2;
    forged[last] = if forged[last] == b'A' { b'B' } else { b'A' };
    api.push(private(557, 1, &format!("/start {}", String::from_utf8(forged).unwrap())));
    api.push(team_reply(80, 4242, "Unrelated."));
    api.push(json!({ "message_id": 1, "date": 0, "chat": { "id": -42, "type": "group" }, "from": { "id": 8, "is_bot": false }, "text": "/start" }));
    api.push(json!({ "message_id": 1, "date": 0, "chat": { "id": 560, "type": "private" }, "from": { "id": 560, "is_bot": true }, "text": "/start" }));
    let last = api.push(private(558, 1, "hello?"));
    api.handled(last).await;
    assert_eq!(api.bot_calls(from), Vec::<String>::new(), "nothing sent");

    // A bare /start from a stranger: one pointer to the site.
    let hello = api.push(private(559, 1, "/start"));
    api.handled(hello).await;
    let replies: Vec<Call> = api.calls()[from..].iter().filter(|c| sent_to(c, "sendMessage", 559)).cloned().collect();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].body["text"], "Send a brief on the site first.");
}

#[tokio::test]
async fn stop_forgets_the_chat() {
    let api = Telegram::start().await;
    let server = with_bot("tg-stop", &api).await;
    let (id, header) = connect(&server, &api, 600, "en").await;
    let from = api.calls().len();
    api.push(private(600, 2, "first message"));
    let copy = api.wait(from, "the copy", |c| sent_to(c, "copyMessage", TEAM)).await;

    let from = api.calls().len();
    api.push(private(600, 3, "/stop"));
    let bye = api.wait(from, "the goodbye", |c| sent_to(c, "sendMessage", 600)).await;
    assert_eq!(bye.body["text"], format!("Disconnected from brief {id}."));
    let note = api.wait(from, "the team's note", |c| team_text(c, &format!("Brief {id}: the client disconnected"))).await;
    assert!(note.body["text"].as_str().unwrap().contains("/stop"), "{}", note.body);
    assert_eq!(note.body["reply_parameters"]["message_id"].as_i64(), Some(header));

    // Gone both ways.
    let from = api.calls().len();
    api.push(private(600, 4, "are you there?"));
    let last = api.push(team_reply(81, copy.id, "Yes!"));
    api.handled(last).await;
    assert_eq!(api.bot_calls(from), Vec::<String>::new(), "nothing relayed after /stop");
}

#[tokio::test]
async fn a_restart_keeps_the_links_and_the_place_in_the_queue() {
    let api = Telegram::start().await;
    let first = with_bot("tg-restart", &api).await;
    let data_dir = first.data_dir.clone();
    let (id, _) = connect(&first, &api, 700, "en").await;
    let from = api.calls().len();
    let update = api.push(private(700, 2, "a message the team answers tomorrow"));
    let copy = api.wait(from, "the copy", |c| sent_to(c, "copyMessage", TEAM)).await;
    api.handled(update).await;
    first.stop().await.unwrap();

    let memory = data_dir.join("telegram.json");
    assert_eq!(std::fs::metadata(&memory).unwrap().permissions().mode() & 0o777, 0o600);
    let text = std::fs::read_to_string(&memory).unwrap();
    assert!(text.contains(&id) && text.contains("700"), "{text}");
    assert!(!text.contains("tomorrow") && !text.contains("Client"), "no message text, no names: {text}");

    let polls = api.state.lock().unwrap().offsets.len();
    let mut cfg = config("tg-restart-2");
    cfg.data_dir = data_dir;
    cfg.telegram = Some(api.settings(TOKEN));
    let _second = TestServer::start(cfg).await;
    let from = api.calls().len();
    api.push(team_reply(90, copy.id, "Good morning."));
    api.wait(from, "the reply after a restart", |c| sent_to(c, "copyMessage", 700) && c.body["message_id"] == 90).await;
    let offsets = api.state.lock().unwrap().offsets.clone();
    assert!(offsets[polls..].iter().all(|o| *o > update), "after the restart it asks from where it stopped: {offsets:?}");
}

#[tokio::test]
async fn answers_in_the_briefs_language_and_tells_the_team_what_failed() {
    let api = Telegram::start().await;
    let server = with_bot("tg-lang", &api).await;
    let (answer, id) = brief(&server, "ru", json!({})).await;
    let from = api.calls().len();
    api.push(private(800, 1, &format!("/start {}", code(&answer))));
    let welcome = api.wait(from, "the answer", |c| sent_to(c, "sendMessage", 800)).await;
    assert_eq!(welcome.body["text"], format!("Чат связан с брифом {id}."));
    // Russian has no goodbye yet: the default language's.
    let from = api.calls().len();
    api.push(private(800, 2, "/stop"));
    let bye = api.wait(from, "the goodbye", |c| sent_to(c, "sendMessage", 800)).await;
    assert_eq!(bye.body["text"], format!("Disconnected from brief {id}."));

    // A client who blocked the bot: the team hears that its reply didn't arrive.
    connect(&server, &api, BLOCKED, "en").await;
    let from = api.calls().len();
    api.push(private(BLOCKED, 5, "a last message before blocking"));
    let copy = api.wait(from, "the copy", |c| sent_to(c, "copyMessage", TEAM)).await;
    let from = api.calls().len();
    api.push(team_reply(91, copy.id, "Here is our answer."));
    let note = api.wait(from, "the team's note", |c| team_text(c, "Not delivered")).await;
    assert!(note.body["text"].as_str().unwrap().contains("blocked the bot"), "{}", note.body);
    assert_eq!(note.body["reply_parameters"]["message_id"].as_i64(), Some(91));
}

#[tokio::test]
async fn chats_are_rate_limited() {
    let api = Telegram::start().await;
    let server = with_bot("tg-limits", &api).await;
    // A stranger: five answers, then silence.
    let from = api.calls().len();
    let mut last = 0;
    for i in 0..8 {
        last = api.push(private(850, i, "/start"));
    }
    api.handled(last).await;
    assert_eq!(api.calls()[from..].iter().filter(|c| sent_to(c, "sendMessage", 850)).count(), 5);

    // A connected client: thirty messages an hour reach the team (its /start counted as a stranger's).
    connect(&server, &api, 851, "en").await;
    let from = api.calls().len();
    for i in 0..35 {
        last = api.push(private(851, 10 + i, &format!("message {i}")));
    }
    api.handled(last).await;
    assert_eq!(api.calls()[from..].iter().filter(|c| sent_to(c, "copyMessage", TEAM)).count(), 30);
}

#[tokio::test]
async fn a_bot_needs_its_messages_in_the_manifest() {
    let api = Telegram::start().await;
    let dist = common::temp_dir("tg-no-strings");
    copy_dir(&common::fixture_dist(), &dist);
    let path = dist.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    manifest.as_object_mut().unwrap().remove("bot");
    std::fs::write(&path, manifest.to_string()).unwrap();
    let mut cfg = config("tg-no-strings");
    cfg.dist = dist;
    cfg.telegram = Some(api.settings(TOKEN));
    let err = match server::Server::bind(cfg).await {
        Ok(_) => panic!("bind should fail"),
        Err(e) => format!("{e:#}"),
    };
    assert!(err.contains("bot-linked") && err.contains("rebuild dist/"), "{err}");
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
