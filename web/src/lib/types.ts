export interface Drone {
	mac: string;
	transport: string;
	rssi: number;
	first_seen_secs_ago: number;
	last_seen_secs_ago: number;
	msg_count: number;
	basic_id: BasicId | null;
	location: Location | null;
	system: DroneSystem | null;
	operator_id: OperatorId | null;
}

export interface BasicId {
	id_type: string;
	ua_type: string;
	ua_id: string;
}

export interface Location {
	status: number;
	direction: number;
	speed_horizontal: number;
	speed_vertical: number;
	latitude: number;
	longitude: number;
	altitude_pressure: number;
	altitude_geodetic: number;
	height_above_takeoff: number;
	timestamp: number;
}

export interface DroneSystem {
	operator_latitude: number;
	operator_longitude: number;
	area_count: number;
	area_radius: number;
	area_ceiling: number;
	area_floor: number;
	classification_type: number;
	operator_altitude_geo: number;
}

export interface OperatorId {
	operator_id_type: number;
	operator_id: string;
}

export interface ScanStatus {
	bluetooth: boolean;
	wifi: boolean;
}
