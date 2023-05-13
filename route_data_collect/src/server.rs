pub mod api_interceptor;
pub mod cache;
pub mod google;

use google::maps::routing::v2::{
    routes_client::RoutesClient, ComputeRoutesRequest, ComputeRoutesResponse, RouteTravelMode,
    RoutingPreference, Units,
};

use tonic::{
    codegen::InterceptedService, service::Interceptor, transport::Channel, Request, Response,
};

pub type GeneralResult = Result<(), Box<dyn std::error::Error>>;

pub async fn get_route(
    client: &mut RoutesClient<InterceptedService<Channel, impl Interceptor>>,
    places: &cache::WaypointCollection,
) -> tonic::Result<Response<ComputeRoutesResponse>> {
    let request = Request::new(ComputeRoutesRequest {
        origin: Some(places.one_utsa_circle().clone()),
        destination: Some(places.fm78_heb().clone()),
        intermediates: vec![
            places.utsa_downtown_campus().clone(),
            places.randolph_park_and_ride().clone(),
            places.train_tracks_on_rittiman_rd().clone(),
        ],
        travel_mode: RouteTravelMode::Drive.into(),
        routing_preference: RoutingPreference::TrafficAwareOptimal.into(),
        language_code: "en-US".to_owned(),
        units: Units::Imperial.into(),
        ..Default::default()
    });

    let response = client.compute_routes(request).await?;
    Ok(response)
}
