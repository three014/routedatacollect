use std::{thread, time::Duration};

use chrono::Local;
use cron::Schedule;
use job_scheduler::scheduler::Scheduler;

fn main() {
    env_logger::init();
    let mut s = Scheduler::<Local>::with_timezone(Local);

    let schedule = "30 * * * * *".parse().unwrap();

    let _id = s.add_job(
        || async {
            println!("Hello World from async job!");
            let _ = "* * *2das".parse::<Schedule>()?;
            Ok(())
        },
        schedule,
        job_scheduler::Limit::NumTimes(5),
    );

    s.start();

    thread::sleep(Duration::from_secs(60));
    let _id1 = s.add_job(
        || async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            println!("Inside the second async job! (Pretend I'm collecting route data!)");
            tokio::time::sleep(Duration::from_secs(3)).await;
            println!("Pretend I panicked lmao");
            panic!();
        },
        "45 * * * * *".parse().unwrap(),
        job_scheduler::Limit::NumTimes(2),
    );

    let mut ids = Vec::new();
    for i in 0..10 {
        ids.push(s.add_job(
            move || async move {
                println!("This is a print statement from index {} of the loop.", i);
                let _ = "5".parse::<i32>()?;
                Ok(())
            },
            "20 * * * * *".parse().unwrap(),
            job_scheduler::Limit::NumTimes(2),
        ));
    }
    thread::sleep(Duration::from_secs(180));
    s.stop();

    println!("Hello, world from main!");
}
