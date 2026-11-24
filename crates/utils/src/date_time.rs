#[allow(dead_code)]
pub mod serial_date_format {
  use jiff::ZonedDateTime;
  use jiff::civil::DateTime;
  use jiff::tz::TimeZone;
  use serde::{self, Deserialize, Deserializer, Serializer};

  const FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

  ///Serialize a `jiff::ZonedDateTime` as its civil (local) date time, without offset nor time zone
  ///
  /// # Errors
  /// If the serialization fails
  pub fn serialize<S>(date: &ZonedDateTime, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let s = date.datetime().strftime(FORMAT).to_string();
    serializer.serialize_str(&s)
  }

  ///Deserialize a `jiff::ZonedDateTime`, reading the string as a date time in the system time zone
  ///
  /// A date time that falls in a DST gap (which [`serialize`] never writes) is shifted forward by the length of the gap,
  /// an ambiguous one (DST fold) is read with the earlier offset
  ///
  /// # Errors
  /// If the deserialization fails
  pub fn deserialize<'de, D>(deserializer: D) -> Result<ZonedDateTime, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    DateTime::strptime(FORMAT, &s)
      .and_then(|dt| dt.to_zoned_datetime(TimeZone::system()))
      .map_err(serde::de::Error::custom)
  }
}

/// Time units, used to express durations (e.g. of a delay step).
pub mod time {
  use gpui_kit::SharedString;
  use gpui_kit::component::select::SelectItem;
  use serde::{Deserialize, Serialize};

  /// A unit of time, serialized by its variant name.
  #[derive(Deserialize, Serialize, Eq, PartialEq, Copy, Clone, Debug)]
  pub enum TimeUnit {
    /// A thousandth of a second
    Millisecond,
    /// A second
    Second,
    /// Sixty seconds
    Minute,
  }

  impl TimeUnit {
    /// Every unit, from the smallest to the largest.
    pub const ALL: [TimeUnit; 3] = [TimeUnit::Millisecond, TimeUnit::Second, TimeUnit::Minute];
  }

  impl SelectItem for TimeUnit {
    type Value = TimeUnit;

    fn title(&self) -> SharedString {
      match self {
        TimeUnit::Millisecond => "Millisecond",
        TimeUnit::Second => "Second",
        TimeUnit::Minute => "Minute",
      }
      .into()
    }

    fn value(&self) -> &Self::Value {
      self
    }
  }
}
