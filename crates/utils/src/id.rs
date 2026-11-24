use gpui_kit::ElementId;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
const LEN: usize = 8;

/// Custom Crockford base32 implementation for short IDs used in project. 8 bytes long
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct KId([u8; LEN]);

impl KId {
  /// Returns the id as a string slice
  pub fn as_str(&self) -> &str {
    std::str::from_utf8(&self.0).expect("KId only holds ASCII characters")
  }

  /// Generates a new random id
  pub fn new() -> Self {
    let mut bytes = [0u8; LEN];
    getrandom::fill(&mut bytes).expect("OS random source unavailable");
    Self(bytes.map(|b| ALPHABET[(b & 31) as usize]))
  }
}

#[cfg(test)]
impl KId {
  /// Builds a constant id repeating the alphabet character at index `v % 32`, meant for tests and fixtures
  pub const fn from_u8(v: u8) -> Self {
    Self([ALPHABET[(v & 31) as usize]; LEN])
  }
}

impl Default for KId {
  fn default() -> Self {
    Self::new()
  }
}

impl Display for KId {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

impl From<KId> for ElementId {
  fn from(value: KId) -> Self {
    Self::Name(value.as_str().into())
  }
}

/// Error returned when parsing a string that is not a valid [`KId`]
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid id '{}': expected {} characters among '{}'", .0, LEN, std::str::from_utf8(ALPHABET).unwrap())]
pub struct KIdParseError(String);

impl FromStr for KId {
  type Err = KIdParseError;

  /// Parses an id from its string form. Case-insensitive
  ///
  /// # Errors
  /// Returns a [`KIdParseError`] if the string is not exactly [`LEN`] characters long, or contains a character outside
  /// [`ALPHABET`]
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let bytes: [u8; LEN] = s
      .as_bytes()
      .to_ascii_lowercase()
      .try_into()
      .map_err(|_| KIdParseError(s.to_owned()))?;
    if bytes.iter().all(|b| ALPHABET.contains(b)) {
      Ok(Self(bytes))
    } else {
      Err(KIdParseError(s.to_owned()))
    }
  }
}

impl Serialize for KId {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.as_str())
  }
}

impl<'de> Deserialize<'de> for KId {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(Error::custom)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use std::collections::BTreeMap;

  #[test]
  fn new_only_uses_alphabet() {
    for _ in 0..1000 {
      let id = KId::new();
      assert_eq!(id.as_str().len(), LEN);
      assert!(id.as_str().bytes().all(|b| ALPHABET.contains(&b)), "{id}");
    }
  }

  #[test]
  fn from_u8_is_valid() {
    assert_eq!(KId::from_u8(1).as_str(), "11111111");
    assert_eq!(KId::from_u8(10).as_str(), "aaaaaaaa");
    // Wraps around the alphabet
    assert_eq!(KId::from_u8(33), KId::from_u8(1));
    for v in 0..=u8::MAX {
      let id = KId::from_u8(v);
      assert_eq!(id.as_str().parse::<KId>(), Ok(id));
    }
  }

  #[test]
  fn display_and_parse_round_trip() {
    let id = KId::new();
    assert_eq!(id.to_string().parse::<KId>(), Ok(id));
  }

  #[test]
  fn parse_rejects_invalid_strings() {
    for invalid in [
      "",
      "1234567",
      "123456789",
      "1234567i", // excluded from the Crockford alphabet
      "1234567l",
      "1234567o",
      "1234567u",
      "1234567-",
      "1234567é", // 8 bytes but non-ASCII
      "c4aa980a-39fe-449f-bf0d-6e0807375f09",
    ] {
      assert!(invalid.parse::<KId>().is_err(), "{invalid:?} should be rejected");
    }
  }

  #[test]
  fn serializes_as_string() {
    let id: KId = "3K9xq2Mt".parse().unwrap();
    assert_eq!(serde_json::to_string(&id).unwrap(), r#""3k9xq2mt""#);
    assert_eq!(serde_json::from_str::<KId>(r#""3k9xq2mt""#).unwrap(), id);
  }

  #[test]
  fn deserialize_rejects_invalid() {
    assert!(serde_json::from_str::<KId>(r#""c4aa980a-39fe-449f-bf0d-6e0807375f09""#).is_err());
    assert!(serde_json::from_str::<KId>("[51,107,57,120,113,50,109,116]").is_err());
    assert!(serde_json::from_str::<KId>("42").is_err());
  }

  #[test]
  fn usable_as_json_map_key() {
    let mut map = BTreeMap::new();
    map.insert(KId::from_u8(1), "a".to_owned());
    map.insert(KId::from_u8(2), "b".to_owned());
    let json = serde_json::to_string(&map).unwrap();
    assert_eq!(json, r#"{"11111111":"a","22222222":"b"}"#);
    assert_eq!(serde_json::from_str::<BTreeMap<KId, String>>(&json).unwrap(), map);
  }
}
