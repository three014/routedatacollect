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

#[derive(Clone, Debug)]
pub struct RouteDataClient {
    client: RoutesClient<InterceptedService<Channel, GoogleRoutesApiInterceptor>>,
}

impl RouteDataClient {
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

    /* Never used, might delete
        pub async fn compute_route_matrix(
            &mut self,
            request: impl tonic::IntoRequest<ComputeRouteMatrixRequest>,
        ) -> tonic::Result<tonic::Response<tonic::codec::Streaming<RouteMatrixElement>>> {
            self.client.compute_route_matrix(request).await
        }
    */
}

pub mod route_data_types {
    use super::{
        db::Location,
        google::maps::routing::v2::{
            ComputeRoutesRequest, RouteTravelMode, RoutingPreference, Units,
        },
    };
    use crate::server::google::maps::routing::v2::{waypoint::LocationType, Waypoint};

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
