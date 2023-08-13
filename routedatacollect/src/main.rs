use crate::server::GeneralResult;
use chrono::NaiveDate;
use job_scheduler::{Limit, Scheduler};
use server::{RouteDataService, Settings};
use std::{io::Write, time::Duration};
use tokio::sync::OnceCell;
use tonic::transport::Channel;

pub mod server;

const SERVER_ADDR: &str = "https://routes.googleapis.com:443";
// Note that setting the field mask to * is OK for
// testing, but discouraged for production.
// For example, for ComputeRoutes, set the field mask to
// "routes.distanceMeters,routes.duration,routes.polyline.encodedPolyline"
// in order to get the route distances, durations, and encoded polylines.
const FIELD_MASK: &str = "routes.distanceMeters,routes.duration,routes.staticDuration";
//const FIELD_MASK: &str = "*";

static API_KEY: OnceCell<String> = OnceCell::const_new();
static DB_URI: OnceCell<String> = OnceCell::const_new();

#[tokio::main(flavor = "current_thread")]
async fn main() -> GeneralResult {
    if let Err(e) = init_logger() {
        eprintln!("Couldn't initialize global logger: {e}");
        return Err(e);
    }
    println!("Program start!");

    if let Err(e) = start().await {
        log::error!("{e}");
        return Err(e);
    }

    Ok(())
}

/// Entry point to the server. Configure database and google api connections, start schedule
/// for pinging Google Routes API for data from UTSA to the HEB on FM-78.
async fn start() -> GeneralResult {
    let api_key = api_key().await?;
    let db_uri = db_uri().await?;

    let mut scheduler = Scheduler::with_timezone(chrono_tz::America::Chicago);
    let every_day_starting_from_school =
        "00 16 13,14,15,16,17,18 * * Mon,Tue,Wed,Thu,Fri".parse()?;

    let channel = Channel::from_static(SERVER_ADDR)
        .timeout(Duration::from_secs(2))
        .keep_alive_while_idle(true)
        .connect()
        .await?;

    let mut svc = RouteDataService::with(Settings {
        channel,
        api_key,
        field_mask: FIELD_MASK,
        connection_uri: Some(db_uri),
    })
    .await?;

    let job = move || async move {
        use crate::server::{RouteDataRequest, WAYPOINT_CACHE};

        let places = &WAYPOINT_CACHE;

        // Create request from Utsa to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until _:43pm
        let req = RouteDataRequest {
            origin: places.one_utsa_circle(),
            destination: places.fm78_heb(),
            intermediates: vec![
                places.crossroads_park_and_ride(),
                places.martin_opposite_leona(),
                places.via_centro_plaza(),
                places.utsa_downtown_campus(),
                places.utsa_san_pedro(),
                places.grand_hyatt(),
                places.randolph_park_and_ride(),
                places.walzem_and_mordred(),
                places.midcrown_ed_white(),
                places.castle_cross_and_castle_hunt(),
                places.train_tracks_on_rittiman_rd(),
            ],
        };

        let response = svc.compute_routes(req).await?;
        svc.save_to_db(response).await?;
        log::info!("Saved \"utsa to heb\" to db!");
        tokio::time::sleep(chrono::Duration::minutes(26).to_std()?).await;

        // Create request from Martin Opposite Leona to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until ++_:25pm
        let req = RouteDataRequest {
            origin: places.martin_opposite_leona(),
            destination: places.fm78_heb(),
            intermediates: vec![
                places.via_centro_plaza(),
                places.utsa_downtown_campus(),
                places.utsa_san_pedro(),
                places.grand_hyatt(),
                places.randolph_park_and_ride(),
                places.walzem_and_mordred(),
                places.midcrown_ed_white(),
                places.castle_cross_and_castle_hunt(),
                places.train_tracks_on_rittiman_rd(),
            ],
        };

        let response = svc.compute_routes(req).await?;
        svc.save_to_db(response).await?;
        log::info!("Saved \"martin opposite leona to heb\" to db!");
        tokio::time::sleep(chrono::Duration::minutes(43).to_std()?).await;

        // Create request from Randolph Park and Ride to Heb
        // Send request, get response
        // Serialize into json, save to database
        // Wait until _:35pm
        let req = RouteDataRequest {
            origin: places.randolph_park_and_ride(),
            destination: places.fm78_heb(),
            intermediates: vec![
                places.walzem_and_mordred(),
                places.midcrown_ed_white(),
                places.castle_cross_and_castle_hunt(),
                places.train_tracks_on_rittiman_rd(),
            ],
        };

        let response = svc.compute_routes(req).await?;
        svc.save_to_db(response).await?;
        log::info!("Saved \"randolph park and ride to heb\" to db!");
        tokio::time::sleep(chrono::Duration::minutes(14).to_std()?).await;

        // Create request from Train Tracks at Rittiman to Heb
        // Send request, get response
        // Serialize into json, save to database
        let req = RouteDataRequest {
            origin: places.castle_cross_and_castle_hunt(),
            destination: places.fm78_heb(),
            intermediates: vec![places.train_tracks_on_rittiman_rd()],
        };

        let response = svc.compute_routes(req).await?;
        svc.save_to_db(response).await?;
        log::info!("Saved \"railroad tracks at rittiman to heb\" to db!");

        Ok(())
    };

    let october_15th = NaiveDate::from_ymd_opt(2023, 10, 15)
        .unwrap()
        .and_hms_opt(13, 0, 0)
        .unwrap();
    scheduler.start().await;
    let _ = scheduler
        .schedule(
            every_day_starting_from_school,
            Some(Limit::EndDate(october_15th)),
            job,
        )
        .await;

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
    scheduler.shutdown().await;

    Ok(())
}

fn init_logger() -> GeneralResult {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {:<5} {:<28} {}",
                chrono::Local::now().format("%m/%d/%Y %H:%M:%S"),
                record.level(),
                record.module_path().unwrap_or(""),
                record.args()
            )
        })
        .target(if let Ok(log_file) = std::env::var("LOG_FILE") {
            let file = std::fs::File::options()
                .append(true)
                .create(true)
                .open(log_file)?;
            let writer = std::io::LineWriter::new(file);
            env_logger::Target::Pipe(Box::new(writer))
        } else {
            env_logger::Target::Stderr
        })
        .init();

    Ok(())
}

async fn api_key() -> Result<&'static str, &'static str> {
    Ok(API_KEY
        .get_or_try_init(|| async {
            std::env::var("API_KEY").map_err(|_| "missing API_KEY environment variable")
        })
        .await?
        .as_str())
}

async fn db_uri() -> Result<&'static str, &'static str> {
    Ok(DB_URI
        .get_or_try_init(|| async {
            std::env::var("CONN_URI")
                .map_err(|_| "missing CONN_URI environment variable for database")
        })
        .await?
        .as_str())
}
