use tonic::{
    metadata::{AsciiMetadataKey, AsciiMetadataValue},
    service::Interceptor,
    Request,
};

pub struct GoogleRoutesApiInterceptor {
    api_key: &'static str,
    field_mask: &'static str,
}
impl GoogleRoutesApiInterceptor {
    const API_KEY_HEADER: &'static str = "X-Goog-Api-Key";
    const FIELD_MASK_HEADER: &'static str = "X-Goog-FieldMask";
    pub fn new(api_key: &'static str, field_mask: &'static str) -> impl Interceptor {
        GoogleRoutesApiInterceptor {
            api_key,
            field_mask,
        }
    }
}
impl Interceptor for GoogleRoutesApiInterceptor {
    fn call(&mut self, mut request: Request<()>) -> tonic::Result<Request<()>> {
        log::info!(target: "api_interceptor", "Intercepting request: {:?}", request);

        match self.api_key.parse::<AsciiMetadataValue>() {
            Ok(api_key) => request.metadata_mut().insert(
                Self::API_KEY_HEADER.parse::<AsciiMetadataKey>().unwrap(),
                api_key,
            ),
            Err(invalid_value) => {
                return Err(tonic::Status::invalid_argument(invalid_value.to_string()))
            }
        };

        match self.field_mask.parse::<AsciiMetadataValue>() {
            Ok(field_mask) => request.metadata_mut().insert(
                Self::FIELD_MASK_HEADER.parse::<AsciiMetadataKey>().unwrap(),
                field_mask,
            ),
            Err(invalid_value) => {
                return Err(tonic::Status::invalid_argument(invalid_value.to_string()))
            }
        };

        Ok(request)
    }
}
