use crate::server::{
    api_interceptor::GoogleRoutesApiInterceptor, cache, get_route,
    google::maps::routing::v2::routes_client::RoutesClient, GeneralResult,
};
use chrono::{DateTime, FixedOffset, Local, NaiveDate};

use job_scheduler::scheduler;
use std::{io, time::Duration};
use tonic::{codegen::InterceptedService, transport::Channel};

mod server;

const SERVER_ADDR: &'static str = "https://routes.googleapis.com:443";
// Note that setting the field mask to * is OK for
// testing, but discouraged for production.
// For example, for ComputeRoutes, set the field mask to
// "routes.distanceMeters,routes.duration,routes.polyline.encodedPolyline"
// in order to get the route distances, durations, and encoded polylines.
const FIELD_MASK: &'static str =
    "routes.distanceMeters,routes.duration,routes.staticDuration,routes.legs";

/// Entry point to the server. Configure database and google api connections, start schedule
/// for pinging Google Routes API for data from UTSA to the HEB on FM-78.
#[tokio::main(flavor = "current_thread")]
async fn main() -> GeneralResult {
    env_logger::init();
    let api_key = std::env::var("API_KEY")?;
    let mut scheduler = scheduler::Scheduler::with_timezone(Local);
    let every_day_starting_from_school = "00 15 13,14,15,16,17 * * *".parse::<cron::Schedule>()?;

    let source_channel = Channel::from_static(SERVER_ADDR)
        .timeout(Duration::from_secs(5))
        .connect()
        .await?;

    let channel_job_copy = source_channel.clone();
    let api_key_job_copy = api_key.clone();
    let fut = || async {
        let mut _client: RoutesClient<InterceptedService<_, _>> =
            RoutesClient::with_interceptor(
                channel_job_copy,
                GoogleRoutesApiInterceptor::new(api_key_job_copy, FIELD_MASK.to_owned()),
            );

        let _places = Box::new(cache::WaypointCollection::new());

        println!("Doing the cool route stuff");

        // Create request from Utsa to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until _:45pm

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
    let tomorrow = NaiveDate::from_ymd_opt(2023, 5, 18)
        .unwrap()
        .and_hms_opt(13, 0, 0)
        .unwrap();
    let tomorrow = chrono::TimeZone::from_local_datetime(&Local, &tomorrow).unwrap();
    scheduler.add_job(
        fut,
        every_day_starting_from_school,
        job_scheduler::Limit::EndDate(tomorrow),
    );

    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async {
        tokio::signal::ctrl_c().await.expect("Failed to listen to event");
        let _ = tx.send(());
    });

    let _ = rx.await;
    scheduler.stop();

    Ok(())
}

fn prompt_for_api_key() -> io::Result<String> {
    eprintln!("Enter API key here: ");
    let mut buf = "".to_owned();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_owned())
}
