use self::{
    api_interceptor::GoogleRoutesApiInterceptor,
    google::maps::routing::v2::{
        routes_client::RoutesClient, ComputeRoutesRequest, ComputeRoutesResponse,
    },
};
use std::time::Duration;
use tonic::{codegen::InterceptedService, transport::Channel};

pub mod api_interceptor;
pub mod cache;
pub mod db;
pub mod google;

pub type GeneralResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Debug)]
pub struct RouteDataClient {
    client: RoutesClient<InterceptedService<Channel, GoogleRoutesApiInterceptor>>,
}

impl RouteDataClient {
    pub fn from_static(s: &'static str, api_key: &str, field_mask: &str) -> RouteDataClient {
        Self {
            client: RoutesClient::with_interceptor(
                Channel::from_static(s)
                    .timeout(Duration::from_secs(5))
                    .connect_lazy(),
                GoogleRoutesApiInterceptor::new(api_key.to_owned(), field_mask.to_owned()),
            ),
        }
    }

    pub async fn compute_routes(
        &mut self,
        request: impl tonic::IntoRequest<ComputeRoutesRequest>,
    ) -> tonic::Result<tonic::Response<ComputeRoutesResponse>> {
        self.client.compute_routes(request).await
    }

    /* Never used, might delete
        pub async fn compute_route_matrix(
            &mut self,
            request: impl tonic::IntoRequest<ComputeRouteMatrixRequest>,
        ) -> tonic::Result<tonic::Response<tonic::codec::Streaming<RouteMatrixElement>>> {
            self.client.compute_route_matrix(request).await
        }
    */
}
