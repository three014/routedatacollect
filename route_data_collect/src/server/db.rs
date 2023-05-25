use crate::server::google::maps::routing::v2::ComputeRoutesResponse;
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializableRouteResponse {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<mongodb::bson::oid::ObjectId>,
    date: DateTimeWrapper,
    response: ComputeRoutesResponse,
}

impl TryFrom<tonic::Response<ComputeRoutesResponse>> for SerializableRouteResponse {
    type Error = String;

    fn try_from(value: tonic::Response<ComputeRoutesResponse>) -> Result<Self, Self::Error> {
        let date = value
            .metadata()
            .get("date")
            .ok_or("Response metadata has no \"date\" field.")?
            .to_str()
            .map_err(|e| e.to_string())?;
        let date = DateTime::parse_from_rfc2822(date).map_err(|e| e.to_string())?;
        Ok(Self {
            id: None,
            date: DateTimeWrapper(date),
            response: value.into_inner(),
        })
    }
}