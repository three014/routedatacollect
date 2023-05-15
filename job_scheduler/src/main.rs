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
            let _ = "* * *2das".parse::<Schedule>()?;
            Ok(())
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

    let mut ids = Vec::new();
    for i in 0..10 {
        ids.push(s.add_job(move || async move {
            println!("This is a print statement from index {} of the loop.", i);
            let _ = "5".parse::<i32>()?;
            Ok(())
        }, "20 * * * * *".parse().unwrap()));
    }
    thread::sleep(Duration::from_secs(60));
    let _ = s.remove_job(id);
    let _ = s.remove_job(id1);
    ids.iter().for_each(|id| { let _ = s.remove_job(*id); });
    s.stop();

    println!("Hello, world from main!");
}
