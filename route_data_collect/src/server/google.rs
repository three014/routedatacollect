pub mod maps {
    pub mod routing {
        pub mod v2 {
            tonic::include_proto!("google.maps.routing.v2");
        }
    }
}

pub mod rpc {
    tonic::include_proto!("google.rpc");
}

pub mod geo {
    pub mod r#type {
        tonic::include_proto!("google.geo.r#type");
    }
}

pub mod r#type {
    tonic::include_proto!("google.r#type");
}
