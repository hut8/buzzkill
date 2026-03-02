CREATE TABLE IF NOT EXISTS sightings (
    id          BIGSERIAL PRIMARY KEY,
    ts          TIMESTAMPTZ NOT NULL DEFAULT now(),
    transport   TEXT NOT NULL,
    mac         MACADDR NOT NULL,
    rssi        SMALLINT,
    msg_type    TEXT NOT NULL,
    counter     SMALLINT,

    -- BasicId
    id_type     TEXT,
    ua_type     TEXT,
    ua_id       TEXT,

    -- Location
    status      SMALLINT,
    direction   DOUBLE PRECISION,
    speed_h     DOUBLE PRECISION,
    speed_v     DOUBLE PRECISION,
    lat         DOUBLE PRECISION,
    lon         DOUBLE PRECISION,
    alt_press   DOUBLE PRECISION,
    alt_geo     DOUBLE PRECISION,
    height_agl  DOUBLE PRECISION,
    loc_ts      DOUBLE PRECISION,

    -- System
    op_lat      DOUBLE PRECISION,
    op_lon      DOUBLE PRECISION,
    area_count  SMALLINT,
    area_radius SMALLINT,
    area_ceil   DOUBLE PRECISION,
    area_floor  DOUBLE PRECISION,
    class_type  SMALLINT,
    op_alt_geo  DOUBLE PRECISION,

    -- OperatorId
    op_id_type  SMALLINT,
    op_id       TEXT,

    -- SelfId
    desc_type   SMALLINT,
    description TEXT,

    -- Auth
    auth_type   SMALLINT,
    auth_page   SMALLINT,
    auth_pages  SMALLINT,
    auth_len    SMALLINT,
    auth_ts     INTEGER,
    auth_data   BYTEA
);

CREATE INDEX idx_sightings_mac ON sightings (mac);
CREATE INDEX idx_sightings_ts  ON sightings (ts);
CREATE INDEX idx_sightings_ua_id ON sightings (ua_id) WHERE ua_id IS NOT NULL;
