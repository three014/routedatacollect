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
            let s = "* * *2das".parse::<Schedule>();
            let guwah = s.expect_err("asdasda");
            let e: Result<(), Box<dyn std::error::Error + Send>> = Err(Box::new(guwah));
            e
        },
        schedule,
    );

    s.start();

    thread::sleep(Duration::from_secs(60));
    let id1 = s.add_job(
        || async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            println!("Inside the second async job! (Pretend I'm collecting route data!)");
            tokio::time::sleep(Duration::from_secs(3)).await;
            println!("Pretend I panicked lmao");
            panic!();
        },
        "45 * * * * *".parse().unwrap(),
    );
    thread::sleep(Duration::from_secs(60));
    let _ = s.remove_job(id);
    let _ = s.remove_job(id1);
    s.stop();

    println!("Hello, world from main!");
}
