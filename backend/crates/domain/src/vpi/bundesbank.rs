//! Parser for the SDMX-JSON response of the Bundesbank time-series API.
//!
//! Only the parsing lives here; fetching is done by the API crate.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use super::VpiValue;

/// One monthly index value from the Bundesbank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VpiDataPoint {
    /// Year of the observation.
    pub year: i32,
    /// Month of the observation, 1–12.
    pub month: u8,
    /// Value rounded to one decimal.
    pub value: VpiValue,
}

/// The response is not SDMX-JSON of the expected shape.
#[derive(Debug, thiserror::Error)]
#[error("Antwort der Bundesbank hat ein unerwartetes Format")]
pub struct VpiParseError(#[source] serde_json::Error);

#[derive(Deserialize)]
struct Response {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    structure: Structure,
    #[serde(rename = "dataSets", default)]
    data_sets: Vec<DataSet>,
}

#[derive(Deserialize)]
struct Structure {
    dimensions: Dimensions,
}

#[derive(Deserialize)]
struct Dimensions {
    #[serde(default)]
    observation: Vec<ObservationDimension>,
}

#[derive(Deserialize)]
struct ObservationDimension {
    #[serde(default)]
    values: Vec<PeriodId>,
}

#[derive(Deserialize)]
struct PeriodId {
    id: String,
}

#[derive(Deserialize)]
struct DataSet {
    #[serde(default)]
    series: BTreeMap<String, Series>,
}

#[derive(Deserialize)]
struct Series {
    #[serde(default)]
    observations: BTreeMap<String, Vec<Value>>,
}

/// Parses the response body into monthly values, sorted by year and month.
///
/// Observations with an unknown period, a period that is not `YYYY-MM` with month
/// 1–12, or a value that is not a number are skipped. Only the first data set is read.
pub fn parse_bundesbank_response(json: &str) -> Result<Vec<VpiDataPoint>, VpiParseError> {
    let response: Response = serde_json::from_str(json).map_err(VpiParseError)?;
    let Some(periods) = response.data.structure.dimensions.observation.first() else {
        return Ok(Vec::new());
    };
    let Some(data_set) = response.data.data_sets.first() else {
        return Ok(Vec::new());
    };
    let mut points: Vec<VpiDataPoint> = data_set
        .series
        .values()
        .flat_map(|series| series.observations.iter())
        .filter_map(|(index, observation)| {
            let period = periods.values.get(index.parse::<usize>().ok()?)?;
            let (year, month) = parse_period(&period.id)?;
            let value = observation_value(observation.first()?)?;
            Some(VpiDataPoint { year, month, value })
        })
        .collect();
    points.sort_by_key(|p| (p.year, p.month));
    Ok(points)
}

/// Reads `YYYY-MM` with a month between 1 and 12.
fn parse_period(id: &str) -> Option<(i32, u8)> {
    let (year, month) = id.split_once('-')?;
    let digits = |s: &str, len: usize| s.len() == len && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(year, 4) || !digits(month, 2) {
        return None;
    }
    let month: u8 = month.parse().ok()?;
    (1..=12)
        .contains(&month)
        .then_some((year.parse().ok()?, month))
}

/// An observation value is a JSON number or a numeric string.
fn observation_value(raw: &Value) -> Option<VpiValue> {
    match raw {
        Value::String(text) => VpiValue::parse(text),
        Value::Number(number) => VpiValue::parse(&number.to_string()),
        _ => None,
    }
}
