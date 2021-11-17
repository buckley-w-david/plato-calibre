pub mod datetime_format {
    use chrono::prelude::*;
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Serializer, Deserializer};

    pub const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s,FORMAT).map_err(serde::de::Error::custom)
        // s.parse::<DateTime<Utc>>().map_err(serde::de::Error::custom)
    }
}


pub mod calibre_datetime_format {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<DateTime<Utc>>().map_err(serde::de::Error::custom)
    }
}

pub mod authors {
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Vec::<String>::deserialize(deserializer)?.join(", "))
    }
}

pub mod identifier {
    use serde::{self, Deserialize, Deserializer};
    use serde_json::Value;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        Value::deserialize(deserializer)?
            .get("url")
            .and_then(Value::as_str)
            .map(|v| v.to_string())
            .ok_or(serde::de::Error::custom("url not found"))
    }
}
