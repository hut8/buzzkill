use std::collections::HashMap;
use std::time::Instant;

use crate::output;
use crate::remoteid::decode::{BasicId, DroneIdMessage, Location, OperatorId, System};

/// Per-drone tracked state, keyed by BLE MAC address.
pub struct DroneState {
    pub mac: [u8; 6],
    pub first_seen: Instant,
    pub last_seen: Instant,
    pub rssi: i8,
    pub last_counter: Option<u8>,
    pub basic_id: Option<BasicId>,
    pub location: Option<Location>,
    pub system: Option<System>,
    pub operator_id: Option<OperatorId>,
    pub msg_count: u64,
}

pub struct Tracker {
    pub drones: HashMap<[u8; 6], DroneState>,
    expiry_secs: u64,
}

impl Tracker {
    pub fn new(expiry_secs: u64) -> Self {
        Self {
            drones: HashMap::new(),
            expiry_secs,
        }
    }

    /// Process a decoded message from a given MAC/RSSI. Returns true if this is a new drone.
    pub fn update(
        &mut self,
        mac: &[u8; 6],
        rssi: i8,
        counter: u8,
        message: &DroneIdMessage,
    ) -> bool {
        let now = Instant::now();
        let is_new = !self.drones.contains_key(mac);

        let state = self.drones.entry(*mac).or_insert_with(|| DroneState {
            mac: *mac,
            first_seen: now,
            last_seen: now,
            rssi,
            last_counter: None,
            basic_id: None,
            location: None,
            system: None,
            operator_id: None,
            msg_count: 0,
        });

        // Dedup by counter byte
        if let Some(last) = state.last_counter {
            if last == counter {
                return false;
            }
        }

        state.last_seen = now;
        state.rssi = rssi;
        state.last_counter = Some(counter);
        state.msg_count += 1;

        match message {
            DroneIdMessage::BasicId(bid) => {
                state.basic_id = Some(bid.clone());
            }
            DroneIdMessage::Location(loc) => {
                state.location = Some(loc.clone());
            }
            DroneIdMessage::System(sys) => {
                state.system = Some(sys.clone());
            }
            DroneIdMessage::OperatorId(oid) => {
                state.operator_id = Some(oid.clone());
            }
            _ => {}
        }

        is_new
    }

    /// Remove drones not seen within the expiry window. Returns MACs of expired drones.
    pub fn expire(&mut self) -> Vec<[u8; 6]> {
        let now = Instant::now();
        let expiry = std::time::Duration::from_secs(self.expiry_secs);
        let mut expired = Vec::new();

        self.drones.retain(|mac, state| {
            if now.duration_since(state.last_seen) > expiry {
                output::print_lost(mac, state);
                expired.push(*mac);
                false
            } else {
                true
            }
        });

        expired
    }
}
