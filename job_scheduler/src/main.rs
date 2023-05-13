use std::{thread, time::Duration};

use chrono::Local;
use cron::Schedule;
use job_scheduler::scheduler::Scheduler;

fn main() {
    env_logger::init();
    let mut s = Scheduler::<Local>::with_timezone(Local);

    let schedule: Schedule = "30 * * * * *".parse().unwrap();

    let id = s.add_job(
        || async {
            println!("Hello World from async job!");
            Ok(())
        },
        schedule,
    );

    s.start();

    thread::sleep(Duration::from_secs(60));
    s.remove_job(id).unwrap();
    thread::sleep(Duration::from_secs(60));
    s.stop();

    println!("Hello, world from main!");
}
