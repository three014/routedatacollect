pub mod maps {
    pub mod routing {
        pub mod v2 {
            include!("../../google_protos/google.maps.routing.v2.rs");
/* 
            impl Serialize for ComputeRoutesResponse {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    let mut state = serializer.serialize_struct("ComputeRoutesResponse", 3)?;
                    state.serialize_field("routes", &self.routes)?;
                    state.serialize_field("fallback_info", &self.fallback_info)?;
                    state.serialize_field("geocoding_results", &self.geocoding_results)?;
                    state.end()
                }
            }

            impl Serialize for Route {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    let mut state = serializer.serialize_struct("Route", 13)?;
                    state.serialize_field("legs", &self.legs)?;
                    state.serialize_field("distance_meters", &self.distance_meters)?;
                    state.serialize_field("duration", &self.duration)?;

                    state.serialize_field("route_labels", &self.route_labels())?;
                    state.end()
                }
            }

            impl Serialize for FallbackInfo {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    todo!()
                }
            }

            impl Serialize for GeocodingResults {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    todo!()
                }
            }
*/
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

/* 
pub mod wrapper {
    use super::maps::routing::v2::{
        routes_client::RoutesClient, ComputeRouteMatrixRequest, ComputeRoutesRequest,
        ComputeRoutesResponse, RouteMatrixElement,
    };
    use crate::server::api_interceptor::GoogleRoutesApiInterceptor;
    use serde::{ser::SerializeStruct, Serialize};
    use tonic::{codegen::InterceptedService, transport::Channel, Response, Streaming};

    pub struct RoutesClientWrapper {
        client: RoutesClient<InterceptedService<Channel, GoogleRoutesApiInterceptor>>,
    }

    impl RoutesClientWrapper {
        pub fn with_interceptor(channel: Channel, interceptor: GoogleRoutesApiInterceptor) -> Self {
            Self {
                client: RoutesClient::with_interceptor(channel, interceptor),
            }
        }

        pub async fn compute_routes(
            &mut self,
            request: impl tonic::IntoRequest<ComputeRoutesRequest>,
        ) -> tonic::Result<Response<ComputeRoutesResponse>> {
            self.client.compute_routes(request).await
        }

        pub async fn compute_route_matrix(
            &mut self,
            request: impl tonic::IntoRequest<ComputeRouteMatrixRequest>,
        ) -> tonic::Result<Response<Streaming<RouteMatrixElement>>> {
            self.client.compute_route_matrix(request).await
        }
    }

    pub struct ComputeRoutesResponseWrapper {
        inner: tonic::Result<Response<ComputeRoutesResponse>>,
    }

    impl Serialize for ComputeRoutesResponseWrapper {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut state = serializer.serialize_struct("ComputeRoutesResponseWrapper", 1)?;
            state.serialize_field("inner", &self.inner.as_ref().unwrap().get_ref())?;
            state.end()
        }
    }
}
*/