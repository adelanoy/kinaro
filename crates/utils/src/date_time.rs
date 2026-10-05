#[allow(dead_code)]
pub mod java_date_format {
  use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
  use serde::{self, Deserialize, Deserializer, Serializer};

  const FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

  ///Serialize a `chrono::DateTime`
  ///
  /// # Errors
  /// If the serialization fails
  pub fn serialize<S>(date: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = format!("{}", date.format(FORMAT));
    serializer.serialize_str(&s)
  }

  ///Deserialize a `chrono::DateTime`, reading the string as a local date time
  ///
  /// A date time that falls in a DST gap (which [`serialize`] never writes) is read as UTC
  ///
  /// # Errors
  /// If the deserialization fails
  pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    let dt = NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
    Ok(
      dt.and_local_timezone(Local)
        .earliest()
        .unwrap_or_else(|| Local.from_utc_datetime(&dt)),
    )
  }
}
