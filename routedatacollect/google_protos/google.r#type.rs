/// An object that represents a latitude/longitude pair. This is expressed as a
/// pair of doubles to represent degrees latitude and degrees longitude. Unless
/// specified otherwise, this must conform to the
/// <a href="<http://www.unoosa.org/pdf/icg/2012/template/WGS_84.pdf">WGS84>
/// standard</a>. Values must be within normalized ranges.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LatLng {
    /// The latitude in degrees. It must be in the range [-90.0, +90.0].
    #[prost(double, tag = "1")]
    pub latitude: f64,
    /// The longitude in degrees. It must be in the range [-180.0, +180.0].
    #[prost(double, tag = "2")]
    pub longitude: f64,
}
/// Localized variant of a text in a particular language.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LocalizedText {
    /// Localized string in the language corresponding to `language_code' below.
    #[prost(string, tag = "1")]
    pub text: ::prost::alloc::string::String,
    /// The text's BCP-47 language code, such as "en-US" or "sr-Latn".
    ///
    /// For more information, see
    /// <http://www.unicode.org/reports/tr35/#Unicode_locale_identifier.>
    #[prost(string, tag = "2")]
    pub language_code: ::prost::alloc::string::String,
}
/// Represents an amount of money with its currency type.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Money {
    /// The three-letter currency code defined in ISO 4217.
    #[prost(string, tag = "1")]
    pub currency_code: ::prost::alloc::string::String,
    /// The whole units of the amount.
    /// For example if `currencyCode` is `"USD"`, then 1 unit is one US dollar.
    #[prost(int64, tag = "2")]
    pub units: i64,
    /// Number of nano (10^-9) units of the amount.
    /// The value must be between -999,999,999 and +999,999,999 inclusive.
    /// If `units` is positive, `nanos` must be positive or zero.
    /// If `units` is zero, `nanos` can be positive, zero, or negative.
    /// If `units` is negative, `nanos` must be negative or zero.
    /// For example $-1.75 is represented as `units`=-1 and `nanos`=-750,000,000.
    #[prost(int32, tag = "3")]
    pub nanos: i32,
}
