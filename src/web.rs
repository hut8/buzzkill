use std::sync::{Arc, Mutex};

use std::time::Instant;

use crate::gps::{self, GpsHandle};
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
    pub gps: GpsHandle,
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
    distance_m: Option<f64>,
    bearing: Option<f64>,
    compass: Option<&'static str>,
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
    let gps_fix = state.gps.lock().ok().and_then(|g| g.clone());
    let now = std::time::Instant::now();

    let mut drones: Vec<DroneJson> = tracker
        .drones
        .values()
        .map(|d| {
            let (distance_m, bearing_deg, compass) = match (&gps_fix, d.location.as_ref()) {
                (Some(fix), Some(loc))
                    if loc.latitude.abs() > 0.0001 || loc.longitude.abs() > 0.0001 =>
                {
                    let dist =
                        gps::haversine_distance(fix.lat, fix.lon, loc.latitude, loc.longitude);
                    let brg = gps::bearing(fix.lat, fix.lon, loc.latitude, loc.longitude);
                    (Some(dist), Some(brg), Some(gps::compass_direction(brg)))
                }
                _ => (None, None, None),
            };
            DroneJson {
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
                distance_m,
                bearing: bearing_deg,
                compass,
            }
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

fn extract_remote_ip(headers: &HeaderMap, remote_addr: Option<std::net::SocketAddr>) -> String {
    if let Some(real_ip) = headers.get("x-real-ip").and_then(|h| h.to_str().ok()) {
        return real_ip.to_string();
    }
    if let Some(forwarded_for) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        if let Some(first_ip) = forwarded_for.split(',').next() {
            return first_ip.trim().to_string();
        }
    }
    remote_addr
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

async fn request_logging(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let remote_ip = extract_remote_ip(
        request.headers(),
        request
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|ci| ci.0),
    );
    let start = Instant::now();

    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status().as_u16();

    log::info!(
        "{} {} {} {:.2}ms {}",
        method,
        path,
        status,
        duration.as_secs_f64() * 1000.0,
        remote_ip
    );

    response
}

fn build_router(state: AppState) -> Router {
    let api_router = Router::new()
        .route("/drones", get(api_drones))
        .route("/status", get(api_status));

    Router::new()
        .nest("/api", api_router)
        .fallback(handle_static_file)
        .with_state(state)
}

pub async fn start_web_server(
    tracker: Arc<Mutex<Tracker>>,
    port: u16,
    scan_config: ScanConfig,
    gps: GpsHandle,
) {
    let state = AppState {
        tracker,
        scan_config,
        gps,
    };

    let app = build_router(state).layer(middleware::from_fn(request_logging));

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to bind web server to port {}: {}", port, e);
            std::process::exit(1);
        });

    log::info!("Web server listening on http://127.0.0.1:{}", port);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap_or_else(|e| {
        log::error!("Web server error: {}", e);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::util::ServiceExt;

    fn test_state() -> AppState {
        AppState {
            tracker: Arc::new(Mutex::new(Tracker::new(60))),
            scan_config: ScanConfig {
                bluetooth: true,
                wifi: false,
            },
            gps: Arc::new(Mutex::new(None)),
        }
    }

    #[tokio::test]
    async fn test_api_drones_empty() {
        let app = build_router(test_state());
        let response = app
            .oneshot(Request::get("/api/drones").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let drones: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(drones.is_empty());
    }

    #[tokio::test]
    async fn test_api_drones_with_data() {
        let state = test_state();
        {
            let mut tracker = state.tracker.lock().unwrap();
            let mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
            let msg = crate::remoteid::decode::DroneIdMessage::BasicId(
                crate::remoteid::decode::BasicId {
                    id_type: crate::remoteid::decode::IdType::SerialNumber,
                    ua_type: crate::remoteid::decode::UaType::HelicopterOrMultirotor,
                    ua_id: "TEST123".to_string(),
                },
            );
            tracker.update(&mac, -60, 1, &msg, "ble");
        }

        let app = build_router(state);
        let response = app
            .oneshot(Request::get("/api/drones").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let drones: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(drones.len(), 1);
        assert_eq!(drones[0]["mac"], "AA:BB:CC:DD:EE:FF");
        assert_eq!(drones[0]["transport"], "ble");
        assert_eq!(drones[0]["rssi"], -60);
        assert_eq!(drones[0]["basic_id"]["ua_id"], "TEST123");
    }

    #[tokio::test]
    async fn test_api_status() {
        let app = build_router(test_state());
        let response = app
            .oneshot(Request::get("/api/status").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(status["bluetooth"], true);
        assert_eq!(status["wifi"], false);
    }

    #[tokio::test]
    async fn test_spa_fallback() {
        let app = build_router(test_state());
        let response = app
            .oneshot(
                Request::get("/some/client/route")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // SPA fallback serves index.html (or 404 if web/build is empty in tests)
        let status = response.status();
        assert!(status == StatusCode::OK || status == StatusCode::NOT_FOUND);
        if status == StatusCode::OK {
            let ct = response.headers().get("content-type").unwrap();
            assert_eq!(ct, "text/html");
        }
    }

    #[tokio::test]
    async fn test_missing_file_with_extension_returns_404() {
        let app = build_router(test_state());
        let response: Response = app
            .oneshot(Request::get("/nonexistent.js").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
