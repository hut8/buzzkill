use std::sync::mpsc;
use std::thread;

use crate::output::format_mac;
use crate::remoteid::decode::DroneIdMessage;

pub struct SightingRow {
    pub transport: &'static str,
    pub mac: String,
    pub rssi: i8,
    pub counter: u8,
    pub msg_type: String,

    // BasicId
    pub id_type: Option<String>,
    pub ua_type: Option<String>,
    pub ua_id: Option<String>,

    // Location
    pub status: Option<i16>,
    pub direction: Option<f64>,
    pub speed_h: Option<f64>,
    pub speed_v: Option<f64>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub alt_press: Option<f64>,
    pub alt_geo: Option<f64>,
    pub height_agl: Option<f64>,
    pub loc_ts: Option<f64>,

    // System
    pub op_lat: Option<f64>,
    pub op_lon: Option<f64>,
    pub area_count: Option<i16>,
    pub area_radius: Option<i16>,
    pub area_ceil: Option<f64>,
    pub area_floor: Option<f64>,
    pub class_type: Option<i16>,
    pub op_alt_geo: Option<f64>,

    // OperatorId
    pub op_id_type: Option<i16>,
    pub op_id: Option<String>,

    // SelfId
    pub desc_type: Option<i16>,
    pub description: Option<String>,

    // Auth
    pub auth_type: Option<i16>,
    pub auth_page: Option<i16>,
    pub auth_pages: Option<i16>,
    pub auth_len: Option<i16>,
    pub auth_ts: Option<i32>,
    pub auth_data: Option<Vec<u8>>,
}

pub fn build_row(
    transport: &'static str,
    mac: &[u8; 6],
    rssi: i8,
    counter: u8,
    msg: &DroneIdMessage,
) -> SightingRow {
    let mut row = SightingRow {
        transport,
        mac: format_mac(mac),
        rssi,
        counter,
        msg_type: msg.msg_type().to_string(),
        id_type: None,
        ua_type: None,
        ua_id: None,
        status: None,
        direction: None,
        speed_h: None,
        speed_v: None,
        lat: None,
        lon: None,
        alt_press: None,
        alt_geo: None,
        height_agl: None,
        loc_ts: None,
        op_lat: None,
        op_lon: None,
        area_count: None,
        area_radius: None,
        area_ceil: None,
        area_floor: None,
        class_type: None,
        op_alt_geo: None,
        op_id_type: None,
        op_id: None,
        desc_type: None,
        description: None,
        auth_type: None,
        auth_page: None,
        auth_pages: None,
        auth_len: None,
        auth_ts: None,
        auth_data: None,
    };

    match msg {
        DroneIdMessage::BasicId(b) => {
            row.id_type = Some(b.id_type.to_string());
            row.ua_type = Some(b.ua_type.to_string());
            row.ua_id = Some(b.ua_id.clone());
        }
        DroneIdMessage::Location(l) => {
            row.status = Some(l.status as i16);
            row.direction = Some(l.direction);
            row.speed_h = Some(l.speed_horizontal);
            row.speed_v = Some(l.speed_vertical);
            row.lat = Some(l.latitude);
            row.lon = Some(l.longitude);
            row.alt_press = Some(l.altitude_pressure);
            row.alt_geo = Some(l.altitude_geodetic);
            row.height_agl = Some(l.height_above_takeoff);
            row.loc_ts = Some(l.timestamp);
        }
        DroneIdMessage::System(s) => {
            row.op_lat = Some(s.operator_latitude);
            row.op_lon = Some(s.operator_longitude);
            row.area_count = Some(s.area_count as i16);
            row.area_radius = Some(s.area_radius as i16);
            row.area_ceil = Some(s.area_ceiling);
            row.area_floor = Some(s.area_floor);
            row.class_type = Some(s.classification_type as i16);
            row.op_alt_geo = Some(s.operator_altitude_geo);
        }
        DroneIdMessage::OperatorId(o) => {
            row.op_id_type = Some(o.operator_id_type as i16);
            row.op_id = Some(o.operator_id.clone());
        }
        DroneIdMessage::SelfId(s) => {
            row.desc_type = Some(s.description_type as i16);
            row.description = Some(s.description.clone());
        }
        DroneIdMessage::Auth(a) => {
            row.auth_type = Some(a.auth_type as i16);
            row.auth_page = Some(a.page_number as i16);
            row.auth_pages = Some(a.page_count as i16);
            row.auth_len = Some(a.length as i16);
            row.auth_ts = Some(a.timestamp as i32);
            row.auth_data = Some(a.data.clone());
        }
        DroneIdMessage::Unknown { .. } => {}
    }

    row
}

pub fn spawn_writer(rx: mpsc::Receiver<SightingRow>) {
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for DB writer");

        rt.block_on(async move {
            let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&database_url)
                .await
                .expect("failed to connect to database");

            // Run migration
            if let Err(e) = sqlx::query(include_str!("../migrations/001_create_sightings.sql"))
                .execute(&pool)
                .await
            {
                log::error!("DB migration failed: {}", e);
                return;
            }

            log::info!("DB writer connected and migration applied");

            while let Ok(row) = rx.recv() {
                if let Err(e) = insert_row(&pool, &row).await {
                    log::error!("DB insert error: {}", e);
                }
            }

            log::info!("DB writer shutting down");
        });
    });
}

async fn insert_row(pool: &sqlx::PgPool, row: &SightingRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sightings (
            transport, mac, rssi, msg_type, counter,
            id_type, ua_type, ua_id,
            status, direction, speed_h, speed_v, lat, lon,
            alt_press, alt_geo, height_agl, loc_ts,
            op_lat, op_lon, area_count, area_radius,
            area_ceil, area_floor, class_type, op_alt_geo,
            op_id_type, op_id,
            desc_type, description,
            auth_type, auth_page, auth_pages, auth_len, auth_ts, auth_data
        ) VALUES (
            $1, $2::macaddr, $3, $4, $5,
            $6, $7, $8,
            $9, $10, $11, $12, $13, $14,
            $15, $16, $17, $18,
            $19, $20, $21, $22,
            $23, $24, $25, $26,
            $27, $28,
            $29, $30,
            $31, $32, $33, $34, $35, $36
        )",
    )
    .bind(row.transport)
    .bind(&row.mac)
    .bind(row.rssi as i16)
    .bind(&row.msg_type)
    .bind(row.counter as i16)
    .bind(&row.id_type)
    .bind(&row.ua_type)
    .bind(&row.ua_id)
    .bind(row.status)
    .bind(row.direction)
    .bind(row.speed_h)
    .bind(row.speed_v)
    .bind(row.lat)
    .bind(row.lon)
    .bind(row.alt_press)
    .bind(row.alt_geo)
    .bind(row.height_agl)
    .bind(row.loc_ts)
    .bind(row.op_lat)
    .bind(row.op_lon)
    .bind(row.area_count)
    .bind(row.area_radius)
    .bind(row.area_ceil)
    .bind(row.area_floor)
    .bind(row.class_type)
    .bind(row.op_alt_geo)
    .bind(row.op_id_type)
    .bind(&row.op_id)
    .bind(row.desc_type)
    .bind(&row.description)
    .bind(row.auth_type)
    .bind(row.auth_page)
    .bind(row.auth_pages)
    .bind(row.auth_len)
    .bind(row.auth_ts)
    .bind(&row.auth_data)
    .execute(pool)
    .await?;

    Ok(())
}
