use std::sync::{Arc, Mutex};

use std::time::Instant;

use crate::output;
use crate::tracker::Tracker;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use include_dir::{include_dir, Dir};
use mime_guess::from_path;
use serde::Serialize;

static ASSETS: Dir<'_> = include_dir!("web/build");

#[derive(Clone, Serialize)]
pub struct ScanConfig {
    pub bluetooth: bool,
    pub wifi: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub tracker: Arc<Mutex<Tracker>>,
    pub scan_config: ScanConfig,
}

#[derive(Serialize)]
struct DroneJson {
    mac: String,
    transport: &'static str,
    rssi: i8,
    first_seen_secs_ago: f64,
    last_seen_secs_ago: f64,
    msg_count: u64,
    basic_id: Option<BasicIdJson>,
    location: Option<LocationJson>,
    system: Option<SystemJson>,
    operator_id: Option<OperatorIdJson>,
}

#[derive(Serialize)]
struct BasicIdJson {
    id_type: String,
    ua_type: String,
    ua_id: String,
}

#[derive(Serialize)]
struct LocationJson {
    status: u8,
    direction: f64,
    speed_horizontal: f64,
    speed_vertical: f64,
    latitude: f64,
    longitude: f64,
    altitude_pressure: f64,
    altitude_geodetic: f64,
    height_above_takeoff: f64,
    timestamp: f64,
}

#[derive(Serialize)]
struct SystemJson {
    operator_latitude: f64,
    operator_longitude: f64,
    area_count: u16,
    area_radius: u16,
    area_ceiling: f64,
    area_floor: f64,
    classification_type: u8,
    operator_altitude_geo: f64,
}

#[derive(Serialize)]
struct OperatorIdJson {
    operator_id_type: u8,
    operator_id: String,
}

async fn api_drones(State(state): State<AppState>) -> impl IntoResponse {
    let tracker = state.tracker.lock().unwrap();
    let now = std::time::Instant::now();

    let mut drones: Vec<DroneJson> = tracker
        .drones
        .values()
        .map(|d| DroneJson {
            mac: output::format_mac(&d.mac),
            transport: d.transport,
            rssi: d.rssi,
            first_seen_secs_ago: now.duration_since(d.first_seen).as_secs_f64(),
            last_seen_secs_ago: now.duration_since(d.last_seen).as_secs_f64(),
            msg_count: d.msg_count,
            basic_id: d.basic_id.as_ref().map(|b| BasicIdJson {
                id_type: format!("{}", b.id_type),
                ua_type: format!("{}", b.ua_type),
                ua_id: b.ua_id.clone(),
            }),
            location: d.location.as_ref().map(|l| LocationJson {
                status: l.status,
                direction: l.direction,
                speed_horizontal: l.speed_horizontal,
                speed_vertical: l.speed_vertical,
                latitude: l.latitude,
                longitude: l.longitude,
                altitude_pressure: l.altitude_pressure,
                altitude_geodetic: l.altitude_geodetic,
                height_above_takeoff: l.height_above_takeoff,
                timestamp: l.timestamp,
            }),
            system: d.system.as_ref().map(|s| SystemJson {
                operator_latitude: s.operator_latitude,
                operator_longitude: s.operator_longitude,
                area_count: s.area_count,
                area_radius: s.area_radius,
                area_ceiling: s.area_ceiling,
                area_floor: s.area_floor,
                classification_type: s.classification_type,
                operator_altitude_geo: s.operator_altitude_geo,
            }),
            operator_id: d.operator_id.as_ref().map(|o| OperatorIdJson {
                operator_id_type: o.operator_id_type,
                operator_id: o.operator_id.clone(),
            }),
        })
        .collect();

    // Sort by most recently seen
    drones.sort_by(|a, b| a.last_seen_secs_ago.total_cmp(&b.last_seen_secs_ago));

    axum::Json(drones)
}

async fn api_status(State(state): State<AppState>) -> impl IntoResponse {
    axum::Json(state.scan_config)
}

async fn handle_static_file(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Handle root path
    if path.is_empty() || path == "index.html" {
        if let Some(file) = ASSETS.get_file("index.html") {
            let mut headers = HeaderMap::new();
            headers.insert("content-type", "text/html".parse().unwrap());
            headers.insert(
                "cache-control",
                "public, max-age=0, must-revalidate".parse().unwrap(),
            );
            return (StatusCode::OK, headers, file.contents()).into_response();
        }
    }

    // Try to find the file in embedded assets
    if let Some(file) = ASSETS.get_file(path) {
        let mut headers = HeaderMap::new();
        let content_type = from_path(path).first_or_octet_stream();
        headers.insert("content-type", content_type.as_ref().parse().unwrap());

        if path.starts_with("_app/") || path.starts_with("assets/") {
            headers.insert(
                "cache-control",
                "public, max-age=31536000, immutable".parse().unwrap(),
            );
        } else {
            headers.insert(
                "cache-control",
                "public, max-age=3600, must-revalidate".parse().unwrap(),
            );
        }

        return (StatusCode::OK, headers, file.contents()).into_response();
    }

    // SPA fallback: serve index.html for client-side routes
    if !path.contains('.') {
        if let Some(file) = ASSETS.get_file("index.html") {
            let mut headers = HeaderMap::new();
            headers.insert("content-type", "text/html".parse().unwrap());
            headers.insert(
                "cache-control",
                "public, max-age=0, must-revalidate".parse().unwrap(),
            );
            return (StatusCode::OK, headers, file.contents()).into_response();
        }
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

async fn request_logging(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status().as_u16();

    log::info!(
        "{} {} {} {:.2}ms",
        method,
        path,
        status,
        duration.as_secs_f64() * 1000.0
    );

    response
}

pub async fn start_web_server(tracker: Arc<Mutex<Tracker>>, port: u16, scan_config: ScanConfig) {
    let state = AppState {
        tracker,
        scan_config,
    };

    let api_router = Router::new()
        .route("/drones", get(api_drones))
        .route("/status", get(api_status));

    let app = Router::new()
        .nest("/api", api_router)
        .fallback(handle_static_file)
        .with_state(state)
        .layer(middleware::from_fn(request_logging));

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to bind web server to port {}: {}", port, e);
            std::process::exit(1);
        });

    log::info!("Web server listening on http://127.0.0.1:{}", port);

    axum::serve(listener, app).await.unwrap_or_else(|e| {
        log::error!("Web server error: {}", e);
    });
}
