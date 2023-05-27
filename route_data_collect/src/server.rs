use self::{
    api_interceptor::GoogleRoutesApiInterceptor,
    db::SerializableRouteResponse,
    google::maps::routing::v2::{routes_client::RoutesClient, ComputeRoutesRequest},
    route_data_types::RouteDataRequest,
};
use tonic::{codegen::InterceptedService, transport::Channel};

pub mod api_interceptor;
pub mod cache;
pub mod db;
pub mod google;

pub type GeneralResult = Result<(), Box<dyn std::error::Error>>;

/// A wrapper around the proto-generated Google Routes API client.
/// Left out the `compute_route_matrix` API call because I didn't
/// need it for this project at the moment. 
/// 
/// Implements `Clone` because the underlying routes client and 
/// channel were intended to have cheap copy implementations.
/// However, the `GoogleRoutesApiInterceptor` that inserts the 
/// API key and field mask into the RPC calls uses `String` 
/// to store that data, making `Clone` not as cheap as it could be.
/// 
/// Ideally I'd store refs to the user data with `&str` instead,
/// but that currently requires a huge overhaul of the job scheduling
/// system that I'm using. I hope to accomplish that soon.
#[derive(Clone, Debug)]
pub struct RouteDataClient {
    client: RoutesClient<InterceptedService<Channel, GoogleRoutesApiInterceptor>>,
}

impl RouteDataClient {

    /// Returns a new `RouteDataClient` from a 
    /// `tonic::transport::Channel`, API key, and
    /// field mask. The inner interceptor
    /// copies the input `&str` values.
    pub fn from_channel_with_key(
        channel: Channel,
        api_key: &str,
        field_mask: &str,
    ) -> RouteDataClient {
        Self {
            client: RoutesClient::with_interceptor(
                channel,
                GoogleRoutesApiInterceptor::new(api_key.to_owned(), field_mask.to_owned()),
            ),
        }
    }

    /// Calls the actual `RoutesClient::compute_routes` method
    /// and returns a serializable version of the `ComputeRoutesResponse`.
    /// 
    /// Accepts a simplified version of the `ComputeRoutesRequest` struct.
    pub async fn compute_routes(
        &mut self,
        request: RouteDataRequest,
    ) -> tonic::Result<SerializableRouteResponse> {
        let origin = request.origin.clone();
        let destination = request.destination.clone();
        let request: ComputeRoutesRequest = request.into();
        let response = self.client.compute_routes(request).await?;
        match SerializableRouteResponse::try_from_response_with_orig_and_dest(
            origin,
            destination,
            response,
        ) {
            Ok(response) => Ok(response),
            Err(e) => Err(tonic::Status::not_found(e)),
        }
    }
}

pub mod route_data_types {
    use super::{
        db::Location,
        google::maps::routing::v2::{
            ComputeRoutesRequest, RouteTravelMode, RoutingPreference, Units,
        },
    };
    use crate::server::google::maps::routing::v2::{waypoint::LocationType, Waypoint};

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
}
