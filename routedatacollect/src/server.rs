pub use self::cache::WaypointCollection;
pub use self::data_types::{Location, RouteDataRequest, SerializableRouteResponse};
use self::{
    api_interceptor::GoogleRoutesApiInterceptor,
    db::AsyncDb,
    google::maps::routing::v2::{routes_client::RoutesClient, ComputeRoutesRequest},
};
use mongodb::results::InsertOneResult;
use std::fmt::Display;
use tonic::{codegen::InterceptedService, transport::Channel};

mod api_interceptor;
mod cache;
mod data_types;
mod db;
mod google;

pub type GeneralResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug)]
pub enum Error {
    SerializeFailed(bson::ser::Error),
    DbNotConnected(&'static str),
    Db(mongodb::error::Error),
    Rpc(tonic::Status),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SerializeFailed(ser) => writeln!(f, "{}", ser),
            Error::DbNotConnected(no_conn) => writeln!(f, "{}", no_conn),
            Error::Db(db) => writeln!(f, "{}", db),
            Error::Rpc(rpc) => writeln!(f, "{}", rpc),
        }
    }
}

impl std::error::Error for Error {}

/// A wrapper around the proto-generated Google Routes API client.
/// Left out the `compute_route_matrix` API call because I didn't
/// need it for this project at the moment.
///
/// Implements `Clone` because the underlying routes client and
/// channel were intended to have cheap copy implementations.
/// Furthermore, the underlying API interceptor only stores
/// references to the API key and field mask, so cloning the
/// entire `RouteDataClient` is still very cheap.
#[derive(Clone, Debug)]
pub struct RouteDataService<'a> {
    client: RoutesClient<InterceptedService<Channel, GoogleRoutesApiInterceptor<'a>>>,
    db: Option<AsyncDb>,
}

impl<'a: 'b, 'b> RouteDataService<'a> {
    pub async fn with(settings: Settings<'a, 'b>) -> Result<RouteDataService<'a>, Error> {
        Ok(Self {
            client: RoutesClient::with_interceptor(
                settings.channel,
                GoogleRoutesApiInterceptor::new(settings.api_key, settings.field_mask),
            ),
            db: match settings.connection_uri {
                Some(uri) => Some(AsyncDb::try_from(uri).await.map_err(Error::Db)?),
                None => None,
            },
        })
    }

    /// Calls the actual `RoutesClient::compute_routes` method
    /// and returns a serializable version of the `ComputeRoutesResponse`.
    ///
    /// Accepts a simplified version of the `ComputeRoutesRequest` struct.
    pub async fn compute_routes(
        &mut self,
        request: RouteDataRequest,
    ) -> Result<SerializableRouteResponse, Error> {
        let origin = request.origin.clone();
        let destination = request.destination.clone();
        let request: ComputeRoutesRequest = request.into();
        let response = self
            .client
            .compute_routes(request)
            .await
            .map_err(Error::Rpc)?;
        match SerializableRouteResponse::try_from_response_with_orig_and_dest(
            origin,
            destination,
            response,
        ) {
            Ok(response) => Ok(response),
            Err(e) => Err(Error::Rpc(tonic::Status::not_found(e))),
        }
    }

    pub async fn save_to_db(
        &self,
        response: SerializableRouteResponse,
    ) -> Result<InsertOneResult, Error> {
        match &self.db {
            Some(db) => Ok(db
                .add_doc(
                    "routes",
                    "utsa_to_heb",
                    bson::to_bson(&response)
                        .map_err(Error::SerializeFailed)?
                        .as_document()
                        .unwrap(),
                )
                .await
                .map_err(Error::Db)?),
            None => Err(Error::DbNotConnected("no database connected to service")),
        }
    }
}

#[derive(Clone)]
pub struct Settings<'a: 'b, 'b> {
    pub channel: Channel,
    pub api_key: &'a str,
    pub field_mask: &'a str,
    pub connection_uri: Option<&'b str>,
}
