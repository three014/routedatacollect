use job_scheduler::{Limit, Scheduler};
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();
    let mut s = Scheduler::new();
    let command = || async {
        println!("Hello World!");
        Ok(())
    };
    let schedule = "00 * * * * *".parse().unwrap();

    let _id = s
        .schedule(schedule, Some(Limit::NumTimes(3)), command)
        .await;

    let _id2 = s
        .schedule(
            "30 * * * * *".parse().unwrap(),
            Some(Limit::NumTimes(5)),
            || async {
                println!("Hello world from async job!!");
                let _: u32 = "asd".parse()?;
                Ok(())
            },
        )
        .await;

    s.start().await;

    tokio::time::sleep(Duration::from_secs(60)).await;

    let _id1 = s
        .schedule(
            "45 * * * * *".parse().unwrap(),
            Some(Limit::NumTimes(2)),
            || async {
                tokio::time::sleep(Duration::from_secs(5)).await;
                println!("Inside the second async job! (Pretend I'm collecting route data!)");
                tokio::time::sleep(Duration::from_secs(3)).await;

                Ok(())
            },
        )
        .await;

    let mut ids = Vec::new();
    for i in 0..10 {
        ids.push(
            s.schedule(
                "20 * * * * *".parse().unwrap(),
                Some(Limit::NumTimes(2)),
                move || async move {
                    println!("This is a print statement from index {} of the loop.", i);
                    let _ = "5".parse::<i32>()?;
                    Ok(())
                },
            )
            .await,
        );
    }

    tokio::time::sleep(Duration::from_secs(120)).await;
    s.shutdown().await;

    println!("Hello, world from main!");
}
