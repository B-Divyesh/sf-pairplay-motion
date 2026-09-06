use std::{
    collections::HashMap,
    env,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::{
        ws::{CloseFrame, Message, WebSocket},
        ConnectInfo, DefaultBodyLimit, Query, State, WebSocketUpgrade,
    },
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use tokio::sync::{mpsc, Mutex, RwLock};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    rooms: Arc<RwLock<HashMap<String, Room>>>,
    db: SqlitePool,
    limits: Arc<RateLimits>,
    static_dir: Arc<PathBuf>,
}

/// A small, in-process token bucket is enough for this single-container relay.
/// It deliberately limits both a source address and the service as a whole so a
/// proxy misconfiguration cannot turn a shared source address into a bypass.
struct RateLimits {
    buckets: Mutex<HashMap<String, TokenBucket>>,
}

struct TokenBucket {
    tokens: f64,
    refreshed_at: Instant,
}

impl RateLimits {
    fn new() -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
        }
    }

    async fn allow(&self, key: String, burst: u32, per_second: f64) -> bool {
        let mut buckets = self.buckets.lock().await;
        let bucket = buckets.entry(key).or_insert_with(|| TokenBucket {
            tokens: burst as f64,
            refreshed_at: Instant::now(),
        });
        let now = Instant::now();
        bucket.tokens = (bucket.tokens
            + now.duration_since(bucket.refreshed_at).as_secs_f64() * per_second)
            .min(burst as f64);
        bucket.refreshed_at = now;
        if bucket.tokens < 1.0 {
            return false;
        }
        bucket.tokens -= 1.0;
        true
    }
}

impl TokenBucket {
    fn fresh(burst: u32) -> Self {
        Self {
            tokens: burst as f64,
            refreshed_at: Instant::now(),
        }
    }

    fn allow(&mut self, burst: u32, per_second: f64) -> bool {
        let now = Instant::now();
        self.tokens = (self.tokens
            + now.duration_since(self.refreshed_at).as_secs_f64() * per_second)
            .min(burst as f64);
        self.refreshed_at = now;
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }
}

struct Room {
    host_token: String,
    host: Option<mpsc::UnboundedSender<Message>>,
    players: HashMap<String, PlayerPeer>,
    created_at: Instant,
}

struct PlayerPeer {
    name: String,
    tx: mpsc::UnboundedSender<Message>,
}

#[derive(Serialize)]
struct RoomCreated {
    code: String,
    host_token: String,
}

#[derive(Deserialize)]
struct SocketParams {
    room: String,
    role: String,
    token: Option<String>,
    name: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pairplay_motion=info,tower_http=info".into()),
        )
        .json()
        .init();

    let database_supplied = env::var_os("DATABASE_URL").is_some();
    let static_supplied = env::var_os("STATIC_DIR").is_some();
    let port_supplied = env::var_os("PORT").is_some();
    let database_url = default_database_url();
    if database_url.starts_with("sqlite:///data/") {
        tokio::fs::create_dir_all("/data").await?;
    } else if database_url.starts_with("sqlite://data/") {
        tokio::fs::create_dir_all("data").await?;
    }
    let options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true)
        .busy_timeout(Duration::from_secs(5));
    let db = SqlitePoolOptions::new()
        // A SQLite file on the product's Azure Files mount has one writer.
        // Keeping one connection also avoids startup lock contention during a
        // single-replica revision replacement.
        .max_connections(1)
        .connect_with(options)
        .await?;
    // Page views are the only durable data and are deliberately non-critical.
    // Do not hold the live relay hostage while an Azure Files mount releases a
    // transient SQLite migration lock during a revision replacement.
    let migration_db = db.clone();
    tokio::spawn(async move {
        if let Err(error) = run_migrations(&migration_db).await {
            warn!(%error, "page-view migration remains unavailable; the relay is serving without it");
        }
    });
    let state = AppState {
        rooms: Arc::new(RwLock::new(HashMap::new())),
        db,
        limits: Arc::new(RateLimits::new()),
        static_dir: Arc::new(PathBuf::from(
            env::var("STATIC_DIR").unwrap_or_else(|_| "dist".into()),
        )),
    };
    let app = build_router(state);
    let port: u16 = env::var("PORT").unwrap_or_else(|_| "8080".into()).parse()?;
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(
        %address,
        database_config = if database_supplied { "supplied" } else if Path::new("/data").is_dir() { "generated-durable" } else { "generated-local" },
        static_config = if static_supplied { "supplied" } else { "generated" },
        port_config = if port_supplied { "supplied" } else { "generated" },
        "PairPlay Motion listening"
    );
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

fn default_database_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| {
        if Path::new("/data").is_dir() {
            "sqlite:///data/pairplay.db?mode=rwc".into()
        } else {
            "sqlite://data/pairplay.db?mode=rwc".into()
        }
    })
}

async fn run_migrations(db: &SqlitePool) -> anyhow::Result<()> {
    const ATTEMPTS: u32 = 20;
    for attempt in 1..=ATTEMPTS {
        match sqlx::migrate!().run(db).await {
            Ok(()) => return Ok(()),
            Err(error) if database_locked(&error) && attempt < ATTEMPTS => {
                warn!(attempt, "SQLite startup lock; retrying migration");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
    unreachable!("the final migration attempt returns above")
}

fn database_locked(error: &sqlx::migrate::MigrateError) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("database is locked") || message.contains("database schema is locked")
}

fn build_router(state: AppState) -> Router {
    let root = state.static_dir.as_ref();
    Router::new()
        .route("/health", get(health))
        .route("/api/rooms", post(create_room))
        .route("/api/page-view", post(page_view))
        .route("/ws", get(ws_handler))
        .route("/", get(serve_spa))
        .route("/demo", get(serve_spa))
        .route("/privacy", get(serve_spa))
        .route("/terms", get(serve_spa))
        .route("/404", get(not_found_spa))
        .route_service("/sw.js", ServeFile::new(root.join("sw.js")))
        .route_service("/icon.svg", ServeFile::new(root.join("icon.svg")))
        .route_service(
            "/manifest.webmanifest",
            ServeFile::new(root.join("manifest.webmanifest")),
        )
        .route_service("/robots.txt", ServeFile::new(root.join("robots.txt")))
        .route_service("/sitemap.xml", ServeFile::new(root.join("sitemap.xml")))
        .nest_service("/assets", ServeDir::new(root.join("assets")))
        .fallback(not_found_spa)
        .layer(DefaultBodyLimit::max(8 * 1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            public_rate_limit,
        ))
        .layer(middleware::from_fn(security_headers))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn serve_spa(State(state): State<AppState>) -> Response {
    match tokio::fs::read(state.static_dir.join("index.html")).await {
        Ok(body) => ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], body).into_response(),
        Err(error) => {
            warn!(%error, "frontend entry file unavailable");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "The game screen is unavailable.",
            )
                .into_response()
        }
    }
}

async fn not_found_spa(State(state): State<AppState>) -> Response {
    let mut response = serve_spa(State(state)).await;
    *response.status_mut() = StatusCode::NOT_FOUND;
    response
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "build": build_identity() }))
}

fn build_identity() -> &'static str {
    option_env!("BUILD_SHA")
        .filter(|value| !value.is_empty())
        .unwrap_or("development")
}

async fn public_rate_limit(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let (scope, source_burst, source_rate, global_burst, global_rate) = match request.uri().path() {
        "/api/rooms" => ("rooms", 12, 0.2, 120, 2.0),
        "/ws" => ("websocket", 30, 0.5, 300, 5.0),
        _ => return next.run(request).await,
    };
    let source = forwarded_source(
        request.headers(),
        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|address| address.0.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
    );
    let source_allowed = state
        .limits
        .allow(
            format!("{scope}:source:{source}"),
            source_burst,
            source_rate,
        )
        .await;
    let global_allowed = state
        .limits
        .allow(format!("{scope}:global"), global_burst, global_rate)
        .await;
    if source_allowed && global_allowed {
        return next.run(request).await;
    }
    warn!(%scope, %source, "public endpoint rate limited");
    let body = if scope == "rooms" {
        Json(json!({ "error": "Too many room requests. Wait a minute and try again." }))
            .into_response()
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many connection attempts. Wait a moment and try again.",
        )
            .into_response()
    };
    let mut response = body;
    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("5"));
    response
}

/// Azure ingress writes the original client as the first X-Forwarded-For hop.
/// Invalid input is ignored so a malformed header cannot create unbounded keys.
fn forwarded_source(headers: &HeaderMap, fallback: String) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .and_then(|value| value.parse::<IpAddr>().ok())
        .map(|address| address.to_string())
        .unwrap_or(fallback)
}

async fn create_room(
    State(state): State<AppState>,
) -> Result<Json<RoomCreated>, (StatusCode, Json<Value>)> {
    let mut rooms = state.rooms.write().await;
    rooms.retain(|_, room| room.created_at.elapsed().as_secs() < 6 * 60 * 60);
    if rooms.len() >= 2_000 {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Room capacity reached. Try again shortly." })),
        ));
    }
    let code = loop {
        let candidate = room_code();
        if !rooms.contains_key(&candidate) {
            break candidate;
        }
    };
    let host_token = Uuid::new_v4().to_string();
    rooms.insert(
        code.clone(),
        Room {
            host_token: host_token.clone(),
            host: None,
            players: HashMap::new(),
            created_at: Instant::now(),
        },
    );
    info!(room = %code, "room created");
    Ok(Json(RoomCreated { code, host_token }))
}

async fn page_view(State(state): State<AppState>) -> StatusCode {
    let result = sqlx::query("INSERT INTO page_views(day, views) VALUES(date('now'), 1) ON CONFLICT(day) DO UPDATE SET views = views + 1")
        .execute(&state.db).await;
    if let Err(error) = result {
        warn!(%error, "page view aggregate failed");
    }
    StatusCode::NO_CONTENT
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<SocketParams>,
    State(state): State<AppState>,
) -> Response {
    let room_code = params.room.trim().to_ascii_uppercase();
    if room_code.len() != 6
        || !room_code
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        return (StatusCode::BAD_REQUEST, "Invalid room code").into_response();
    }
    ws.max_message_size(8 * 1024)
        .on_upgrade(move |socket| socket_session(socket, state, room_code, params))
}

async fn socket_session(
    socket: WebSocket,
    state: AppState,
    room_code: String,
    params: SocketParams,
) {
    if params.role != "host" && params.role != "controller" {
        close_socket(socket, 4000, "Unknown role").await;
        return;
    }
    if params.role == "controller" && !valid_name(params.name.as_deref().unwrap_or("")) {
        close_socket(socket, 4000, "Player name is required").await;
        return;
    }

    let is_host = params.role == "host";
    let player_id = if is_host {
        "host".into()
    } else {
        Uuid::new_v4().simple().to_string()[..8].to_string()
    };
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let admission_error = {
        let mut rooms = state.rooms.write().await;
        match rooms.get_mut(&room_code) {
            None => Some((4004, "Room not found")),
            Some(room) if is_host && params.token.as_deref() != Some(room.host_token.as_str()) => {
                Some((4001, "Host token rejected"))
            }
            Some(room) if !is_host && room.players.len() >= 4 => Some((4003, "Room is full")),
            Some(room) => {
                if is_host {
                    room.host = Some(tx.clone());
                } else {
                    room.players.insert(
                        player_id.clone(),
                        PlayerPeer {
                            name: params.name.clone().unwrap_or_default(),
                            tx: tx.clone(),
                        },
                    );
                }
                None
            }
        }
    };
    if let Some((code, reason)) = admission_error {
        close_socket(socket, code, reason).await;
        return;
    }

    let (mut socket_tx, mut socket_rx) = socket.split();
    send_json(
        &tx,
        json!({ "type": "welcome", "player_id": player_id, "room": room_code }),
    );
    broadcast_roster(&state, &room_code).await;

    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if socket_tx.send(message).await.is_err() {
                break;
            }
        }
    });

    let mut relay_limiter = TokenBucket::fresh(30);
    while let Some(Ok(message)) = socket_rx.next().await {
        let Message::Text(text) = message else {
            continue;
        };
        if !relay_limiter.allow(30, 15.0) {
            warn!(room = %room_code, peer = %player_id, "websocket relay rate limited");
            break;
        }
        let Ok(mut payload) = serde_json::from_str::<Value>(text.as_str()) else {
            continue;
        };
        let kind = payload.get("type").and_then(Value::as_str).unwrap_or("");
        if is_host {
            if !matches!(
                kind,
                "calibrate" | "game_start" | "cue" | "scoreboard" | "round_end"
            ) {
                continue;
            }
            let recipients = {
                let rooms = state.rooms.read().await;
                rooms
                    .get(&room_code)
                    .map(|room| {
                        room.players
                            .values()
                            .map(|peer| peer.tx.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            };
            for recipient in recipients.iter() {
                send_json(recipient, payload.clone());
            }
        } else {
            if !matches!(kind, "calibrated" | "motion") {
                continue;
            }
            if let Some(object) = payload.as_object_mut() {
                object.insert("playerId".into(), Value::String(player_id.clone()));
            }
            let host = {
                let rooms = state.rooms.read().await;
                rooms.get(&room_code).and_then(|room| room.host.clone())
            };
            if let Some(host) = host {
                send_json(&host, payload);
            }
        }
    }

    writer.abort();
    {
        let mut rooms = state.rooms.write().await;
        if let Some(room) = rooms.get_mut(&room_code) {
            if is_host {
                room.host = None;
            } else {
                room.players.remove(&player_id);
            }
        }
    }
    broadcast_roster(&state, &room_code).await;
    info!(room = %room_code, peer = %player_id, "peer disconnected");
}

async fn close_socket(mut socket: WebSocket, code: u16, reason: &str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.to_owned().into(),
        })))
        .await;
}

async fn broadcast_roster(state: &AppState, room_code: &str) {
    let (recipients, players) = {
        let rooms = state.rooms.read().await;
        let Some(room) = rooms.get(room_code) else {
            return;
        };
        let mut recipients = room
            .players
            .values()
            .map(|peer| peer.tx.clone())
            .collect::<Vec<_>>();
        if let Some(host) = &room.host {
            recipients.push(host.clone());
        }
        let players = room
            .players
            .iter()
            .map(|(id, peer)| json!({ "id": id, "name": peer.name }))
            .collect::<Vec<_>>();
        (recipients, players)
    };
    let payload = json!({ "type": "roster", "players": players });
    for recipient in recipients.iter() {
        send_json(recipient, payload.clone());
    }
}

fn send_json(tx: &mpsc::UnboundedSender<Message>, payload: Value) {
    if let Ok(text) = serde_json::to_string(&payload) {
        let _ = tx.send(Message::Text(text.into()));
    }
}

fn room_code() -> String {
    const ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    let mut rng = rand::rng();
    (0..6)
        .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
        .collect()
}

fn valid_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty() && trimmed.chars().count() <= 24 && !trimmed.chars().any(char::is_control)
}

async fn security_headers(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("accelerometer=(self), gyroscope=(self), payment=()"),
    );
    headers.insert(header::HeaderName::from_static("content-security-policy"), HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ws: wss: https://api.sociobot.in https://pilot-api.sociobot.in; frame-ancestors 'none'; base-uri 'self'; form-action 'self' https://api.sociobot.in"));
    if path.starts_with("/assets/") || path == "/icon.svg" {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    } else if path == "/sw.js" {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    }
    response
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    info!("graceful shutdown requested");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    async fn test_state() -> AppState {
        let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        AppState {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            db,
            limits: Arc::new(RateLimits::new()),
            static_dir: Arc::new(PathBuf::from("dist")),
        }
    }

    #[test]
    fn codes_avoid_ambiguous_characters() {
        for _ in 0..100 {
            let code = room_code();
            assert_eq!(code.len(), 6);
            assert!(!code.contains(['0', '1', 'I', 'O']));
        }
    }

    #[test]
    fn player_names_are_bounded() {
        assert!(valid_name("Ada"));
        assert!(!valid_name(""));
        assert!(!valid_name("\n"));
        assert!(!valid_name(&"x".repeat(25)));
    }

    #[tokio::test]
    async fn health_route_has_an_explicit_non_placeholder_identity() {
        let app = build_router(test_state().await);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["build"], build_identity());
        assert!(!build_identity().is_empty());
    }

    #[tokio::test]
    async fn room_route_creates_unique_room() {
        let state = test_state().await;
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/rooms")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.rooms.read().await.len(), 1);
    }

    #[tokio::test]
    async fn room_creation_is_rate_limited_with_a_retry_hint() {
        let app = build_router(test_state().await);
        for _ in 0..12 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/rooms")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/rooms")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(header::RETRY_AFTER).unwrap(), "5");
    }

    #[tokio::test]
    async fn room_limit_uses_the_first_forwarded_client_and_keeps_clients_isolated() {
        let app = build_router(test_state().await);
        for _ in 0..12 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/rooms")
                        .header("x-forwarded-for", "198.51.100.44, 10.0.0.2")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
        let limited = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/rooms")
                    .header("x-forwarded-for", "198.51.100.44, 10.0.0.2")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(limited.headers().get(header::RETRY_AFTER).unwrap(), "5");

        let other_client = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/rooms")
                    .header("x-forwarded-for", "203.0.113.18")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(other_client.status(), StatusCode::OK);
    }

    #[test]
    fn websocket_relay_bucket_has_a_bounded_burst() {
        let mut bucket = TokenBucket::fresh(30);
        assert!((0..30).all(|_| bucket.allow(30, 15.0)));
        assert!(!bucket.allow(30, 15.0));
    }

    #[tokio::test]
    async fn page_view_stores_only_daily_count() {
        let state = test_state().await;
        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/page-view")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let views: i64 = sqlx::query_scalar("SELECT views FROM page_views")
            .fetch_one(&state.db)
            .await
            .unwrap();
        assert_eq!(views, 1);
    }
}
