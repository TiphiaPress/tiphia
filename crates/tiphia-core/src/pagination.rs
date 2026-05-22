use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct PaginationQuery {
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    pub page: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    pub per_page: Option<u64>,
}

impl PaginationQuery {
    pub fn page(&self) -> u64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> u64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }
}

fn deserialize_optional_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptionalNumber {
        Number(u64),
        Text(String),
        Empty,
    }

    match Option::<OptionalNumber>::deserialize(deserializer)? {
        Some(OptionalNumber::Number(value)) => Ok(Some(value)),
        Some(OptionalNumber::Text(value)) if value.trim().is_empty() => Ok(None),
        Some(OptionalNumber::Text(value)) => value
            .parse::<u64>()
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(OptionalNumber::Empty) | None => Ok(None),
    }
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub meta: PageMeta,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PageMeta {
    pub page: u64,
    pub per_page: u64,
    pub total: u64,
    pub total_pages: u64,
}

impl<T> Page<T> {
    pub fn new(data: Vec<T>, page: u64, per_page: u64, total: u64, total_pages: u64) -> Self {
        Self {
            data,
            meta: PageMeta {
                page,
                per_page,
                total,
                total_pages,
            },
        }
    }
}
