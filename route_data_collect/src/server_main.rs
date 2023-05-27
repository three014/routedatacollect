use crate::server::GeneralResult;
use bson::Document;
use chrono::NaiveDate;
use job_scheduler::scheduler::Scheduler;
use mongodb::{options::ClientOptions, Client};
use std::{io::Write, time::Duration};
use tonic::transport::Channel;

mod server;

const SERVER_ADDR: &str = "https://routes.googleapis.com:443";
// Note that setting the field mask to * is OK for
// testing, but discouraged for production.
// For example, for ComputeRoutes, set the field mask to
// "routes.distanceMeters,routes.duration,routes.polyline.encodedPolyline"
// in order to get the route distances, durations, and encoded polylines.
const FIELD_MASK: &str = "routes.distanceMeters,routes.duration,routes.staticDuration";
//const FIELD_MASK: &str = "*";

/// Entry point to the server. Configure database and google api connections, start schedule
/// for pinging Google Routes API for data from UTSA to the HEB on FM-78.
#[tokio::main(flavor = "current_thread")]
async fn main() -> GeneralResult {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {:<5} {:<28} {}",
                chrono::Local::now().format("%d/%m/%Y %H:%M:%S"),
                record.level(),
                record.module_path().unwrap_or(""),
                record.args()
            )
        })
        .init();
    let api_key = std::env::var("API_KEY").map_err(|_| "Missing API_KEY environment variable")?;
    let mut scheduler = Scheduler::with_timezone(chrono_tz::America::Chicago);
    let every_day_starting_from_school = "00 16 13,14,15,16,17 * * *".parse::<cron::Schedule>()?;

    let channel = Channel::from_static(SERVER_ADDR)
        .timeout(Duration::from_secs(2))
        .keep_alive_while_idle(true)
        .connect()
        .await?;
    let mongo_uri = std::env::var("CONN_URI")
        .map_err(|_| "Missing CONN_URI environment variable for mongodb")?;
    let mongo = Client::with_options(ClientOptions::parse_async(mongo_uri).await?)?;
    let utsa_to_heb = mongo
        .database("routes")
        .collection::<Document>("utsa_to_heb");

    let fut = move || async move {
        use crate::server::{
            cache::WaypointCollection, route_data_types::RouteDataRequest, RouteDataClient,
        };
        let mut client =
            RouteDataClient::from_channel_with_key(channel.clone(), api_key.as_str(), FIELD_MASK);

        let places = Box::new(WaypointCollection::new());

        println!("Doing the cool route stuff");

        // Create request from Utsa to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until _:43pm
        let req = RouteDataRequest {
            origin: places.one_utsa_circle().clone(),
            destination: places.fm78_heb().clone(),
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
                places.castle_cross_and_castle_hunt().clone(),
                places.train_tracks_on_rittiman_rd().clone(),
            ],
        };

        let response = client.compute_routes(req).await?;
        utsa_to_heb
            .insert_one(bson::to_bson(&response)?.as_document().unwrap(), None)
            .await?;
        tokio::time::sleep(chrono::Duration::minutes(26).to_std()?).await;

        // Create request from Martin Opposite Leona to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until ++_:25pm
        let req = RouteDataRequest {
            origin: places.martin_opposite_leona().clone(),
            destination: places.fm78_heb().clone(),
            intermediates: vec![
                places.via_centro_plaza().clone(),
                places.utsa_downtown_campus().clone(),
                places.utsa_san_pedro().clone(),
                places.grand_hyatt().clone(),
                places.randolph_park_and_ride().clone(),
                places.walzem_and_mordred().clone(),
                places.midcrown_ed_white().clone(),
                places.castle_cross_and_castle_hunt().clone(),
                places.train_tracks_on_rittiman_rd().clone(),
            ],
        };

        let response = client.compute_routes(req).await?;
        utsa_to_heb
            .insert_one(bson::to_bson(&response)?.as_document().unwrap(), None)
            .await?;
        tokio::time::sleep(chrono::Duration::minutes(43).to_std()?).await;

        // Create request from Randolph Park and Ride to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until _:35pm
        let req = RouteDataRequest {
            origin: places.randolph_park_and_ride().clone(),
            destination: places.fm78_heb().clone(),
            intermediates: vec![
                places.walzem_and_mordred().clone(),
                places.midcrown_ed_white().clone(),
                places.castle_cross_and_castle_hunt().clone(),
                places.train_tracks_on_rittiman_rd().clone(),
            ],
        };

        let response = client.compute_routes(req).await?;
        utsa_to_heb
            .insert_one(bson::to_bson(&response)?.as_document().unwrap(), None)
            .await?;
        tokio::time::sleep(chrono::Duration::minutes(14).to_std()?).await;

        // Create request from Train Tracks at Rittiman to Heb
        // Send request, get response
        // Serialize into json, save to database
        let req = RouteDataRequest {
            origin: places.castle_cross_and_castle_hunt().clone(),
            destination: places.fm78_heb().clone(),
            intermediates: vec![places.train_tracks_on_rittiman_rd().clone()],
        };

        let response = client.compute_routes(req).await?;
        utsa_to_heb
            .insert_one(bson::to_bson(&response)?.as_document().unwrap(), None)
            .await?;

        Ok(())
    };

    scheduler.start();
    let october_15th = NaiveDate::from_ymd_opt(2023, 10, 15)
        .unwrap()
        .and_hms_opt(13, 0, 0)
        .unwrap();
    scheduler.add_job(
        fut,
        every_day_starting_from_school,
        job_scheduler::Limit::EndDate(october_15th),
    );

    // Shutdown listeners
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
    scheduler.stop();

    Ok(())
}
