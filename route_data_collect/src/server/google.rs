#[allow(clippy::all)]
pub mod maps {
    pub mod routing {
        pub mod v2 {
            include!("../../google_protos/google.maps.routing.v2.rs");
        }
    }
}

#[allow(clippy::all)]
pub mod rpc {
    include!("../../google_protos/google.rpc.rs");
}

#[allow(clippy::all)]
pub mod geo {
    pub mod r#type {
        include!("../../google_protos/google.geo.r#type.rs");
    }
}

#[allow(clippy::all)]
pub mod r#type {
    include!("../../google_protos/google.r#type.rs");
}

#[allow(clippy::all)]
pub mod protobuf {
    include!("../../google_protos/google.protobuf.rs");
}
