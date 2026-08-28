use std::{
    collections::HashMap, env, net::SocketAddr, path::Path, str::FromStr, sync::Arc, time::Instant,
};

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        DefaultBodyLimit, Query, State, WebSocketUpgrade,
    },
    http::{header, HeaderValue, Request, StatusCode},
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
use tokio::sync::{mpsc, RwLock};
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

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/pairplay.db?mode=rwc".into());
    if database_url.starts_with("sqlite://data/") {
        tokio::fs::create_dir_all("data").await?;
    }
    let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::migrate!().run(&db).await?;
    let state = AppState {
        rooms: Arc::new(RwLock::new(HashMap::new())),
        db,
    };
    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "dist".into());
    let app = build_router(state, &static_dir);
    let port: u16 = env::var("PORT").unwrap_or_else(|_| "8080".into()).parse()?;
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(%address, "PairPlay Motion listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn build_router(state: AppState, static_dir: &str) -> Router {
    let root = Path::new(static_dir);
    Router::new()
        .route("/health", get(health))
        .route("/api/rooms", post(create_room))
        .route("/api/page-view", post(page_view))
        .route("/ws", get(ws_handler))
        .route_service("/sw.js", ServeFile::new(root.join("sw.js")))
        .route_service("/icon.svg", ServeFile::new(root.join("icon.svg")))
        .route_service(
            "/manifest.webmanifest",
            ServeFile::new(root.join("manifest.webmanifest")),
        )
        .route_service("/robots.txt", ServeFile::new(root.join("robots.txt")))
        .route_service("/sitemap.xml", ServeFile::new(root.join("sitemap.xml")))
        .nest_service("/assets", ServeDir::new(root.join("assets")))
        .fallback_service(ServeFile::new(root.join("index.html")))
        .layer(DefaultBodyLimit::max(8 * 1024))
        .layer(middleware::from_fn(security_headers))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "build": option_env!("BUILD_SHA").unwrap_or("development") }))
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
    {
        let rooms = state.rooms.read().await;
        let Some(room) = rooms.get(&room_code) else {
            return (StatusCode::NOT_FOUND, "Room not found").into_response();
        };
        if params.role == "host" {
            if params.token.as_deref() != Some(room.host_token.as_str()) {
                return (StatusCode::FORBIDDEN, "Host token rejected").into_response();
            }
        } else if params.role == "controller" {
            if room.players.len() >= 4 {
                return (StatusCode::CONFLICT, "Room is full").into_response();
            }
            if !valid_name(params.name.as_deref().unwrap_or("")) {
                return (StatusCode::BAD_REQUEST, "Player name is required").into_response();
            }
        } else {
            return (StatusCode::BAD_REQUEST, "Unknown role").into_response();
        }
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
    let (mut socket_tx, mut socket_rx) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let is_host = params.role == "host";
    let player_id = if is_host {
        "host".into()
    } else {
        Uuid::new_v4().simple().to_string()[..8].to_string()
    };

    {
        let mut rooms = state.rooms.write().await;
        let Some(room) = rooms.get_mut(&room_code) else {
            return;
        };
        if is_host {
            room.host = Some(tx.clone());
        } else {
            room.players.insert(
                player_id.clone(),
                PlayerPeer {
                    name: params.name.unwrap_or_default(),
                    tx: tx.clone(),
                },
            );
        }
    }
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

    while let Some(Ok(message)) = socket_rx.next().await {
        let Message::Text(text) = message else {
            continue;
        };
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
    async fn health_route_works() {
        let app = build_router(test_state().await, "dist");
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
    }

    #[tokio::test]
    async fn room_route_creates_unique_room() {
        let state = test_state().await;
        let app = build_router(state.clone(), "dist");
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
    async fn page_view_stores_only_daily_count() {
        let state = test_state().await;
        let app = build_router(state.clone(), "dist");
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
