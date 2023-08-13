use std::borrow::Cow;
use super::data_types::Location;

pub static WAYPOINT_CACHE: WaypointCollection = WaypointCollection::new();

/// An immutable collection of frequently used waypoints for my project.
/// Very large struct, would recommend passing this by reference until
/// you need a particular waypoint, then cloning that waypoint.
pub struct WaypointCollection {
    one_utsa_circle: Location,
    crossroads_park_and_ride: Location,
    martin_opposite_leona: Location,
    via_centro_plaza: Location,
    utsa_downtown_campus: Location,
    utsa_san_pedro: Location,
    grand_hyatt: Location,
    randolph_park_and_ride: Location,
    walzem_and_mordred: Location,
    midcrown_ed_white: Location,
    castle_cross_and_castle_hunt: Location,
    train_tracks_on_rittiman_rd: Location,
    fm78_heb: Location,
}

impl WaypointCollection {
    pub const fn new() -> WaypointCollection {
        WaypointCollection {
            one_utsa_circle: Location {
                address: Cow::Borrowed("One UTSA Circle, San Antonio, TX 78249, USA"),
                place_id: Cow::Borrowed("ChIJh705pGFmXIYR6o_rMARBOsw"),
            },
            crossroads_park_and_ride: Location {
                address: Cow::Borrowed("Crossroads Park & Ride, Balcones Heights, TX 78201, USA"),
                place_id: Cow::Borrowed("ChIJw2IJsT9eXIYR2fua_adlYFQ")
            },
            martin_opposite_leona: Location {
                address: Cow::Borrowed("Martin Opposite Leona, San Antonio, TX 78207, USA"),
                place_id: Cow::Borrowed("ChIJ3ayu3UtfXIYRqXtAsRt-ZA8")
            },
            via_centro_plaza: Location {
                address: Cow::Borrowed("909 W Houston St, San Antonio, TX 78207, USA"),
                place_id: Cow::Borrowed("ChIJAYhDWEpfXIYRwtu8lNZWDkc")
            },
            utsa_downtown_campus: Location {
                address: Cow::Borrowed("501 W César E Chávez Blvd, San Antonio, TX 78207, USA"),
                place_id: Cow::Borrowed("ChIJHXXwLEtfXIYRIMdj4wpcYRA")
            },
            utsa_san_pedro: Location {
                address: Cow::Borrowed("506 Dolorosa St, San Antonio, TX 78204, USA"), 
                place_id: Cow::Borrowed("ChIJZ5ztQv5ZXIYRFl3Bupk6PVQ")
            },
            grand_hyatt: Location {
                address: Cow::Borrowed("600 E Market St, San Antonio, TX 78205, USA"),
                place_id: Cow::Borrowed("ChIJy6ciXqpYXIYRo5XoO_IClA8")
            },
            randolph_park_and_ride: Location {
                address: Cow::Borrowed("Randolph Park and Ride, San Antonio, TX 78233, USA"),
                place_id: Cow::Borrowed("ChIJMct5BLKNXIYRxcHtoTFl5K4")
            },
            walzem_and_mordred: Location {
                address: Cow::Borrowed("Walzem & Mordred, Windcrest, TX 78218, USA"),
                place_id: Cow::Borrowed("ChIJY5YTj1nzXIYRnG48q_P195A")
            },
            midcrown_ed_white: Location {
                address: Cow::Borrowed("Midcrown Between Round Table & Prince Valiant, San Antonio, TX 78218, USA"),
                place_id: Cow::Borrowed("ChIJTzCwmUPzXIYR4iNYswS_4Dg")
            },
            castle_cross_and_castle_hunt: Location {
                address: Cow::Borrowed("Castle Cross & Castle Hunt, San Antonio, TX 78218, USA"),
                place_id: Cow::Borrowed("ChIJuVHGigzzXIYREiAXfeeKMFM")
            },
            train_tracks_on_rittiman_rd: Location {
                address: Cow::Borrowed("5 Rittiman Cut, San Antonio, TX 78218, USA"),
                place_id: Cow::Borrowed("Eio1IFJpdHRpbWFuIEN1dCwgU2FuIEFudG9uaW8sIFRYIDc4MjE4LCBVU0EiMBIuChQKEgmP8VGVC_NchhHy8qA2Kg7EjBAFKhQKEgnTQwyVC_NchhG3sjHzhR2CpQ")
            },
            fm78_heb: Location {
                address: Cow::Borrowed("6580 Farm-To-Market Rd 78, San Antonio, TX 78244, USA"),
                place_id: Cow::Borrowed("ChIJKY2HNwfzXIYRIfkmIOxSHY4")
            },
        }
    }

    pub fn one_utsa_circle(&self) -> &Location {
        &self.one_utsa_circle
    }

    pub fn crossroads_park_and_ride(&self) -> &Location {
        &self.crossroads_park_and_ride
    }

    pub fn martin_opposite_leona(&self) -> &Location {
        &self.martin_opposite_leona
    }

    pub fn via_centro_plaza(&self) -> &Location {
        &self.via_centro_plaza
    }

    pub fn utsa_downtown_campus(&self) -> &Location {
        &self.utsa_downtown_campus
    }

    pub fn utsa_san_pedro(&self) -> &Location {
        &self.utsa_san_pedro
    }

    pub fn grand_hyatt(&self) -> &Location {
        &self.grand_hyatt
    }

    pub fn randolph_park_and_ride(&self) -> &Location {
        &self.randolph_park_and_ride
    }

    pub fn walzem_and_mordred(&self) -> &Location {
        &self.walzem_and_mordred
    }

    pub fn midcrown_ed_white(&self) -> &Location {
        &self.midcrown_ed_white
    }

    pub fn castle_cross_and_castle_hunt(&self) -> &Location {
        &self.castle_cross_and_castle_hunt
    }

    pub fn train_tracks_on_rittiman_rd(&self) -> &Location {
        &self.train_tracks_on_rittiman_rd
    }

    pub fn fm78_heb(&self) -> &Location {
        &self.fm78_heb
    }
}

impl Default for WaypointCollection {
    fn default() -> Self {
        Self::new()
    }
}
