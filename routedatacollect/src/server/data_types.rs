use super::google::maps::routing::v2::{
    waypoint::LocationType, ComputeRoutesRequest, ComputeRoutesResponse, RouteTravelMode,
    RoutingPreference, Units, Waypoint,
};
use chrono::{DateTime, FixedOffset};
use serde::{de::Visitor, Deserialize, Serialize};

struct DateTimeWrapper(DateTime<FixedOffset>);

impl Serialize for DateTimeWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.to_rfc2822().as_str())
    }
}

impl<'de> Deserialize<'de> for DateTimeWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DateTimeVisitor;

        impl<'de> Visitor<'de> for DateTimeVisitor {
            type Value = DateTimeWrapper;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("date in rfc2822 format")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match DateTime::parse_from_rfc2822(v) {
                    Ok(date) => Ok(DateTimeWrapper(date)),
                    Err(_) => Err(E::invalid_value(serde::de::Unexpected::Str(v), &self)),
                }
            }
        }
        deserializer.deserialize_newtype_struct("DateTimeWrapper", DateTimeVisitor)
    }
}

/// A simplified version of the `ComputeRoutesRequest` struct used
/// for Google's Routes API.
pub struct RouteDataRequest {
    pub origin: Location,
    pub destination: Location,
    pub intermediates: Vec<Location>,
}

impl From<RouteDataRequest> for ComputeRoutesRequest {
    fn from(value: RouteDataRequest) -> ComputeRoutesRequest {
        ComputeRoutesRequest {
            origin: Some(Waypoint {
                location_type: Some(LocationType::PlaceId(value.origin.place_id)),
                ..Default::default()
            }),
            destination: Some(Waypoint {
                location_type: Some(LocationType::PlaceId(value.destination.place_id)),
                ..Default::default()
            }),
            intermediates: value
                .intermediates
                .into_iter()
                .map(|location| Waypoint {
                    location_type: Some(LocationType::PlaceId(location.place_id)),
                    ..Default::default()
                })
                .collect(),
            routing_preference: RoutingPreference::TrafficAwareOptimal.into(),
            travel_mode: RouteTravelMode::Drive.into(),
            units: Units::Imperial.into(),
            language_code: "en-US".to_owned(),
            ..Default::default()
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializableRouteResponse {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<mongodb::bson::oid::ObjectId>,
    origin: Location,
    destination: Location,
    date: DateTimeWrapper,
    response: ComputeRoutesResponse,
}

impl SerializableRouteResponse {
    pub fn try_from_response_with_orig_and_dest(
        origin: Location,
        destination: Location,
        response: tonic::Response<ComputeRoutesResponse>,
    ) -> Result<Self, String> {
        let date = response
            .metadata()
            .get("date")
            .ok_or("Response metadata has no \"date\" field.")?
            .to_str()
            .map_err(|e| e.to_string())?;
        let date = DateTime::parse_from_rfc2822(date).map_err(|e| e.to_string())?;
        Ok(Self {
            id: None,
            date: DateTimeWrapper(date),
            origin,
            destination,
            response: response.into_inner(),
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Location {
    pub address: String,
    pub place_id: String,
}
