use std::{collections::BTreeMap, path::PathBuf};

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{
  Serialize, Serializer,
  de::{self, Deserializer, MapAccess, SeqAccess, Visitor},
};

pub type Mapping = BTreeMap<String, ValueWithSource>;
pub type Sequence = Vec<ValueWithSource>;
pub type FlattenValues = BTreeMap<String, (String, PathBuf)>;

#[derive(Clone, Debug)]
pub enum ScalarType {
  String,
  Integer,
  Float,
  Boolean,
}

#[derive(Clone, Debug)]
pub enum ValueWithSource {
  Null,
  Scalar {
    value: String,
    source: PathBuf,
    original_type: ScalarType,
  },
  Mapping(Mapping),
  Sequence(Sequence),
}

impl ValueWithSource {
  pub fn is_mapping(&self) -> bool { matches!(self, ValueWithSource::Mapping(_)) }

  pub fn is_sequence(&self) -> bool { matches!(self, ValueWithSource::Sequence(_)) }

  pub fn as_sequence_mut(&mut self) -> Option<&mut Sequence> {
    match self {
      ValueWithSource::Sequence(s) => Some(s),
      _ => None,
    }
  }

  pub fn as_mapping_mut(&mut self) -> Option<&mut Mapping> {
    match self {
      ValueWithSource::Mapping(m) => Some(m),
      _ => None,
    }
  }
}

impl Serialize for ValueWithSource {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    match self {
      ValueWithSource::Null => serializer.serialize_none(),
      ValueWithSource::Scalar {
        value, original_type, ..
      } => {
        // Use the original type to serialize correctly
        match original_type {
          ScalarType::Boolean => {
            if let Ok(b) = value.parse::<bool>() {
              serializer.serialize_bool(b)
            } else {
              // Fallback to string if parsing fails
              serializer.serialize_str(value)
            }
          }
          ScalarType::Integer => {
            if let Ok(i) = value.parse::<i64>() {
              serializer.serialize_i64(i)
            } else {
              // Fallback to string if parsing fails
              serializer.serialize_str(value)
            }
          }
          ScalarType::Float => {
            if let Ok(f) = value.parse::<f64>() {
              serializer.serialize_f64(f)
            } else {
              // Fallback to string if parsing fails
              serializer.serialize_str(value)
            }
          }
          ScalarType::String => serializer.serialize_str(value),
        }
      }
      ValueWithSource::Mapping(m) => {
        let mut map = serializer.serialize_map(Some(m.len()))?;
        for (k, v) in m {
          map.serialize_entry(k, v)?;
        }
        map.end()
      }
      ValueWithSource::Sequence(s) => {
        let mut seq = serializer.serialize_seq(Some(s.len()))?;
        for item in s {
          seq.serialize_element(item)?;
        }
        seq.end()
      }
    }
  }
}

struct ConfigFileVisitor {
  source: PathBuf,
}

pub fn config_file_expected_message() -> String {
  struct Expected(ConfigFileVisitor);

  impl std::fmt::Display for Expected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.expecting(f) }
  }

  format!("{}", Expected(ConfigFileVisitor { source: PathBuf::new() }))
}

impl<'de> Visitor<'de> for ConfigFileVisitor {
  type Value = ValueWithSource;

  fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "any YAML value") }

  fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
    Ok(ValueWithSource::Scalar {
      value: v.to_string(),
      source: self.source,
      original_type: ScalarType::Boolean,
    })
  }

  fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
    Ok(ValueWithSource::Scalar {
      value: v.to_string(),
      source: self.source,
      original_type: ScalarType::Integer,
    })
  }

  fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
    Ok(ValueWithSource::Scalar {
      value: v.to_string(),
      source: self.source,
      original_type: ScalarType::Integer,
    })
  }

  fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
    Ok(ValueWithSource::Scalar {
      value: v.to_string(),
      source: self.source,
      original_type: ScalarType::Float,
    })
  }

  fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
    Ok(ValueWithSource::Scalar {
      value: v.to_string(),
      source: self.source,
      original_type: ScalarType::String,
    })
  }

  fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> { Ok(ValueWithSource::Null) }

  fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
    let mut items = Vec::new();

    while let Some(item) = seq.next_element_seed(ConfigFile {
      source: self.source.clone(),
    })? {
      items.push(item);
    }

    Ok(ValueWithSource::Sequence(items))
  }

  fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
    let mut entries = Mapping::new();

    while let Some(key) = map.next_key::<String>()? {
      let value = map.next_value_seed(ConfigFile {
        source: self.source.clone(),
      })?;
      entries.insert(key, value);
    }

    Ok(ValueWithSource::Mapping(entries))
  }
}

pub struct ConfigFile {
  pub source: PathBuf,
}

impl<'de> de::DeserializeSeed<'de> for ConfigFile {
  type Value = ValueWithSource;

  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    deserializer.deserialize_any(ConfigFileVisitor { source: self.source })
  }
}
