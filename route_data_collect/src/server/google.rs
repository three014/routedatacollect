pub mod maps {
    pub mod routing {
        pub mod v2 {
            include!("../../google_protos/google.maps.routing.v2.rs");
        }
    }
}

pub mod rpc {
    include!("../../google_protos/google.rpc.rs");
}

pub mod geo {
    pub mod r#type {
        include!("../../google_protos/google.geo.r#type.rs");
    }
}

pub mod r#type {
    include!("../../google_protos/google.r#type.rs");
}

pub mod protobuf {
    include!("../../google_protos/google.protobuf.rs");
}
