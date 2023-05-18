use super::google::maps::routing::v2::{waypoint::LocationType, Waypoint};

/// An immutable collection of frequently used waypoints for my project.
/// Very large struct, would recommend passing this by reference until
/// you need a particular waypoint, then cloning that waypoint.
pub struct WaypointCollection {
    one_utsa_circle: Waypoint,
    crossroads_park_and_ride: Waypoint,
    martin_opposite_leona: Waypoint,
    via_centro_plaza: Waypoint,
    utsa_downtown_campus: Waypoint,
    utsa_san_pedro: Waypoint,
    grand_hyatt: Waypoint,
    randolph_park_and_ride: Waypoint,
    walzem_and_mordred: Waypoint,
    midcrown_ed_white: Waypoint,
    train_tracks_on_rittiman_rd: Waypoint,
    fm78_heb: Waypoint,
}

impl WaypointCollection {
    pub fn new() -> WaypointCollection {
        WaypointCollection {
            one_utsa_circle: Waypoint {
                location_type: Some(LocationType::PlaceId(
                    "ChIJh705pGFmXIYR6o_rMARBOsw".to_owned(),
                )),
                ..Default::default()
            },
            crossroads_park_and_ride: Waypoint {
                location_type: Some(LocationType::PlaceId("ChIJw2IJsT9eXIYR2fua_adlYFQ".to_owned())),
                ..Default::default()
            },
            martin_opposite_leona: Waypoint {
                location_type: Some(LocationType::PlaceId("ChIJ3ayu3UtfXIYRqXtAsRt".to_owned())),
                ..Default::default()
            },
            via_centro_plaza: Waypoint {
                location_type: Some(LocationType::PlaceId("ChIJAYhDWEpfXIYRwtu8lNZWDkc".to_owned())),
                ..Default::default()
            },
            utsa_downtown_campus: Waypoint {
                location_type: Some(LocationType::PlaceId(
                    "ChIJHXXwLEtfXIYRIMdj4wpcYRA".to_owned(),
                )),
                ..Default::default()
            },
            utsa_san_pedro: Waypoint {
                location_type: Some(LocationType::PlaceId("ChIJZ5ztQv5ZXIYRFl3Bupk6PVQ".to_owned())),
                ..Default::default()
            },
            grand_hyatt: Waypoint {
                location_type: Some(LocationType::PlaceId("ChIJy6ciXqpYXIYRo5XoO_IClA8".to_owned())),
                via: true,
                ..Default::default()
            },
            randolph_park_and_ride: Waypoint {
                location_type: Some(LocationType::PlaceId(
                    "ChIJMct5BLKNXIYRxcHtoTFl5K4".to_owned(),
                )),
                ..Default::default()
            },
            walzem_and_mordred: Waypoint {
                location_type: Some(LocationType::PlaceId("ChIJY5YTj1nzXIYRnG48q_P195A".to_owned())),
                ..Default::default()
            },
            midcrown_ed_white: Waypoint {
                location_type: Some(LocationType::PlaceId("ChIJTzCwmUPzXIYR4iNYswS_4Dg".to_owned())),
                ..Default::default()
            },
            train_tracks_on_rittiman_rd: Waypoint {
                location_type: Some(LocationType::PlaceId(
                    "Eio1IFJpdHRpbWFuIEN1dCwgU2FuIEFudG9uaW8sIFRYIDc4MjE4LCBVU0EiMBIuChQKEgmP8VGVC_NchhHy8qA2Kg7EjBAFKhQKEgnTQwyVC_NchhG3sjHzhR2CpQ".to_owned(),
                )),
                via: true,
                ..Default::default()
            },
            fm78_heb: Waypoint {
                location_type: Some(LocationType::PlaceId(
                    "ChIJKY2HNwfzXIYRIfkmIOxSHY4".to_owned(),
                )),
                ..Default::default()
            },
        }
    }

    pub fn one_utsa_circle(&self) -> &Waypoint {
        &self.one_utsa_circle
    }

    pub fn utsa_downtown_campus(&self) -> &Waypoint {
        &self.utsa_downtown_campus
    }

    pub fn utsa_san_pedro(&self) -> &Waypoint {
        &self.utsa_san_pedro
    }

    pub fn randolph_park_and_ride(&self) -> &Waypoint {
        &self.randolph_park_and_ride
    }

    pub fn train_tracks_on_rittiman_rd(&self) -> &Waypoint {
        &self.train_tracks_on_rittiman_rd
    }

    pub fn fm78_heb(&self) -> &Waypoint {
        &self.fm78_heb
    }
}
