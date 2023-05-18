use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home_dir = PathBuf::from(env::var("HOME").unwrap());
    let google_dir = home_dir.join("google/googleapis");

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        //.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile(
            &[
                google_dir.join("google/maps/routing/v2/route.proto"),
                google_dir.join("google/maps/routing/v2/routes_service.proto"),
            ],
            &[google_dir],
        )?;

    Ok(())
}
