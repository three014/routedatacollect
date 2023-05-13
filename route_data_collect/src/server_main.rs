use crate::server::{
    api_interceptor::GoogleRoutesApiInterceptor, cache, get_route,
    google::maps::routing::v2::routes_client::RoutesClient, GeneralResult,
};
use std::time::Duration;
use tonic::{codegen::InterceptedService, transport::Channel};

mod server;

const SERVER_ADDR: &'static str = "https://routes.googleapis.com:443";
const API_KEY: &'static str = "AIzaSyBog1xzPe-BJQaK5fkMEUPixqvlnVKtxSw";
// Note that setting the field mask to * is OK for
// testing, but discouraged for production.
// For example, for ComputeRoutes, set the field mask to
// "routes.distanceMeters,routes.duration,routes.polyline.encodedPolyline"
// in order to get the route distances, durations, and encoded polylines.
const FIELD_MASK: &'static str =
    "routes.distanceMeters,routes.duration,routes.staticDuration,routes.legs";

/// Entry point to the server. Configure database and google api connections, start schedule
/// for pinging Google Routes API for data from UTSA to the HEB on FM-78.
#[tokio::main]
async fn main() -> GeneralResult {
    env_logger::init();

    let channel = Channel::from_static(SERVER_ADDR)
        .timeout(Duration::from_secs(2))
        .connect()
        .await?;

    let mut client: RoutesClient<InterceptedService<Channel, _>> = RoutesClient::with_interceptor(
        channel,
        GoogleRoutesApiInterceptor::new(API_KEY, FIELD_MASK),
    );

    let places = cache::WaypointCollection::new();

    let route = get_route(&mut client, &places).await?;
    log::info!("Response: {:#?}", route);

    Ok(())
}
