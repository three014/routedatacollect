use crate::server::{
    api_interceptor::GoogleRoutesApiInterceptor, cache,
    google::maps::routing::v2::routes_client::RoutesClient, GeneralResult,
};
use chrono::{Local, NaiveDate};

use job_scheduler::scheduler;
use std::time::Duration;
use tonic::{codegen::InterceptedService, transport::Channel};

mod server;

const SERVER_ADDR: &str = "https://routes.googleapis.com:443";
// Note that setting the field mask to * is OK for
// testing, but discouraged for production.
// For example, for ComputeRoutes, set the field mask to
// "routes.distanceMeters,routes.duration,routes.polyline.encodedPolyline"
// in order to get the route distances, durations, and encoded polylines.
const FIELD_MASK: &str = "routes.distanceMeters,routes.duration,routes.staticDuration,routes.legs";
//const FIELD_MASK: &str = "*";

/// Entry point to the server. Configure database and google api connections, start schedule
/// for pinging Google Routes API for data from UTSA to the HEB on FM-78.
#[tokio::main(flavor = "current_thread")]
async fn main() -> GeneralResult {
    env_logger::init();
    let api_key = std::env::var("API_KEY")?;
    let mut scheduler = scheduler::Scheduler::with_timezone(Local);
    let every_day_starting_from_school = "00 15 13,14,15,16,17 * * *".parse::<cron::Schedule>()?;

    //let redis = redis::Client::open("redis://db")?;
    //let mut con = redis.get_async_connection().await?;

    log::debug!("Connected to redis.");

    let google_routes = Channel::from_static(SERVER_ADDR)
        .timeout(Duration::from_secs(5))
        .connect()
        .await?;

    log::debug!("Connected to Google Routes API.");

    let routes_job_copy = google_routes.clone();
    let api_key_job_copy = api_key.clone();
    let fut = || async {
        use crate::server::google::maps::routing::v2::{
            ComputeRoutesRequest, RouteTravelMode, RoutingPreference, Units
        };
        let mut client: RoutesClient<InterceptedService<Channel, GoogleRoutesApiInterceptor>> =
            RoutesClient::with_interceptor(
                routes_job_copy,
                GoogleRoutesApiInterceptor::new(api_key_job_copy, FIELD_MASK.to_owned()),
            );

        let places = Box::new(cache::WaypointCollection::new());

        println!("Doing the cool route stuff");

        // Create request from Utsa to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until _:45pm
        let req = tonic::Request::new(ComputeRoutesRequest {
            origin: Some(places.one_utsa_circle().clone()),
            destination: Some(places.fm78_heb().clone()),
            intermediates: vec![
                places.crossroads_park_and_ride().clone(),
                places.martin_opposite_leona().clone(),
                places.via_centro_plaza().clone(),
                places.utsa_downtown_campus().clone(),
                places.utsa_san_pedro().clone(),
                places.grand_hyatt().clone(),
                places.randolph_park_and_ride().clone(),
                places.walzem_and_mordred().clone(),
                places.midcrown_ed_white().clone(),
                places.train_tracks_on_rittiman_rd().clone(),
            ],
            routing_preference: RoutingPreference::TrafficAwareOptimal.into(),
            travel_mode: RouteTravelMode::Drive.into(),
            units: Units::Imperial.into(),
            language_code: "en-US".to_owned(),
            ..Default::default()
        });

        let response = client.compute_routes(req).await?;

        println!("{}", serde_json::to_string_pretty(&response.into_inner())?);

        // Create request from Martin Opposite Leona to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until ++_:15pm

        // Create request from Randolph Park and Ride to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until _:30pm

        // Create request from Train Tracks at Rittiman to Heb
        // Send request, get response
        // Serialize into json, save to database

        Ok(())
    };

    scheduler.start();
    let tomorrow = NaiveDate::from_ymd_opt(2023, 5, 22)
        .unwrap()
        .and_hms_opt(13, 0, 0)
        .unwrap();
    let tomorrow = chrono::TimeZone::from_local_datetime(&Local, &tomorrow).unwrap();
    scheduler.add_job(
        fut.clone(),
        every_day_starting_from_school,
        job_scheduler::Limit::EndDate(tomorrow),
    );

    scheduler.add_job(
        fut,
        "30 * * * * *".parse()?,
        job_scheduler::Limit::NumTimes(1),
    );

    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::interrupt()).unwrap();
        let mut sigint = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = sigterm.recv() => tx.send(()),
            _ = sigint.recv() => tx.send(()),
        }
    });

    let _ = rx.await;
    //scheduler.stop();

    Ok(())
}
