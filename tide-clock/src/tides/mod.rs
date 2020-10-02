use chrono::{DateTime, Duration, Local, Utc};
use ordered_float::OrderedFloat;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{error::Error, fs};

use crate::maths;

#[tokio::main]
pub async fn load_tides_from_api() -> Result<TideResponse, Box<dyn Error>> {
    let settings: Settings = load_config("resources/Settings.toml")?;
    let secrets: Secrets = load_config("resources/Secrets.toml")?;

    let url = format!(
        "https://www.worldtides.info/api/v2?heights&extremes&datum={}&days=3&lat={}&lon={}&step={}&key={}",
        settings.datum, settings.lat, settings.lon, settings.step, secrets.key
    );

    //Call result from api into dynamic json object (to preserve all fields)
    let json: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|err| simple_error::SimpleError::new(format!("Check wifi connection! {}", err)))?
        .json()
        .await?;

    //Write the raw json to disk. This can help debug some issues that might break parsing, eg auth failure
    let write_result = fs::write("resources/tides.json", json.to_string());
    if write_result.is_err() {
        println!(
            "Could not write json artefact to 'resources/tides.json'. Err {}",
            write_result.err().unwrap()
        );
    }

    //Parse dynamic json to typed data
    let response: TideResponse = serde_json::from_value(json)?;

    Ok(response)
}

fn load_config<T>(path: &str) -> Result<T, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    let rawdata = fs::read_to_string(path)
        .map_err(|err| simple_error::SimpleError::new(format!("{1} Filename {0}:", path, err)))?;

    let result = toml::from_str(&rawdata)?;

    Ok(result)
}

pub fn local_to_utc(dt: DateTime<Local>) -> DateTime<Utc> {
    //No idea is this is the canonically correct way
    let utc: DateTime<Utc> = dt.with_timezone(&Utc);

    utc
}

#[derive(Deserialize, Serialize)]
pub struct Secrets {
    pub key: String,
}

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub lon: String,
    pub lat: String,
    pub step: String,
    pub datum: String,
}

pub struct TideModel {
    water_mark: WaterMarkData,
    normalised_heights: Vec<f32>,
    dates: Vec<DateTime<Utc>>,
    extremes: Vec<TideExtremeGraphData>,
}

pub enum DataFreshness {
    Fresh,
    NeedsUpdate,
}

pub struct TideModelWindow<'a> {
    pub water_mark: WaterMarkData,
    pub normalised_heights: &'a [f32],
    pub dates: &'a [DateTime<Utc>],
    extremes: &'a [TideExtremeGraphData],
    start_index: u32,
}

impl TideModel {
    pub fn new(data: TideResponse) -> TideModel {
        let water_mark = TideModel::get_water_mark(&data.heights);

        //Iterate over all heights
        //Transform the height to a nomralised value
        //Collect (aka allocate) into a new collection
        let normalised_heights: Vec<f32> = data
            .heights
            .iter()
            .map(|h| maths::inverse_lerp(h.height, water_mark.low_water, water_mark.high_water))
            .collect();

        let dates = data.heights.iter().map(|h| h.date).collect();

        let mut extremes: Vec<TideExtremeGraphData> = vec![];

        for extreme in data.extremes.iter() {
            //Iterate over all heights
            //Enumerate them to preserve the index (index:usize, height:TideHeightData)
            //Find the height.dt closest to extreme.dt - min_by_key returns smallest delta
            let option = data
                .heights
                .iter()
                .enumerate()
                .min_by_key(|kvp| find_abs_diff(kvp.1.dt, extreme.dt));

            //If Some, returns the iterator tuple (index:usize, height:TideHeightData)
            if let Some((index, _height_data)) = option {
                extremes.push(TideExtremeGraphData {
                    index: index as u32,
                    date: extreme.date,
                });
            }
        }

        TideModel {
            water_mark,
            normalised_heights,
            extremes,
            dates,
        }
    }

    pub fn get_window(&self, now: DateTime<Local>) -> (TideModelWindow, DataFreshness) {
        //let start_utc = local_to_utc(start_local);
        let start_utc = local_to_utc(now)
            .checked_sub_signed(Duration::hours(8))
            .unwrap_or_else(|| {
                eprintln!("Failed to substract 8 hours from time {:?}", now);
                local_to_utc(now)
            });

        let mut freshness = DataFreshness::Fresh;
        let mut start: usize = 0;
        match TideModel::find_time_index(&self.dates, start_utc) {
            Some(start_index) => {
                start = start_index as usize;
                //Do we have enough values to draw the graph? 112 = 107 (width of display) + 5 frames
                if self.normalised_heights.len() - start < 112 {
                    freshness = DataFreshness::NeedsUpdate;
                }
            }
            None => freshness = DataFreshness::NeedsUpdate,
        };

        (
            TideModelWindow {
                water_mark: self.water_mark,
                normalised_heights: &self.normalised_heights[start..],
                dates: &self.dates[start..],
                extremes: &self.extremes,
                start_index: start as u32,
            },
            freshness,
        )
    }

    fn get_water_mark(heights: &[TideHeightData]) -> WaterMarkData {
        //Need to use ordered float as the default doesn't implement Ord
        let high_water = match heights.iter().map(|h| OrderedFloat::from(h.height)).max() {
            Some(ordered) => ordered.into_inner(),
            None => 0_f32,
        };

        //Need to use ordered float as the default doesn't implement Ord
        let low_water = match heights.iter().map(|h| OrderedFloat::from(h.height)).min() {
            Some(ordered) => ordered.into_inner(),
            None => 0_f32,
        };

        WaterMarkData {
            high_water,
            low_water,
            current_water: 2.0,
        }
    }

    pub fn find_time_index(dates: &[DateTime<Utc>], now: DateTime<Utc>) -> Option<u32> {
        //For each date in dates
        //Enumerate as (index, date)
        //Find the entry whos dateTime is closest to now
        let result = dates.iter().enumerate().min_by_key(|kvp| {
            let delta = now.signed_duration_since(kvp.1.to_owned());
            delta.num_seconds().abs()
        });

        if let Some(kvp) = result {
            return Some(kvp.0 as u32);
        }

        None
    }

    pub fn get_date_range(&self) -> Option<(&DateTime<Utc>, &DateTime<Utc>)> {
        if let Some(first) = self.dates.first() {
            if let Some(last) = self.dates.last() {
                return Some((first, last));
            }
        }

        None
    }

    pub fn get_current_norm_height(&self, now: DateTime<Utc>) -> f32 {
        if let Some(index) = TideModel::find_time_index(&self.dates, now) {
            if let Some(height) = self.normalised_heights.get(index as usize) {
                return height.to_owned();
            }
        }

        // Essentially marker will be at zero
        -10_f32
    }
}

impl TideModelWindow<'_> {
    pub fn extremes(&self) -> &[TideExtremeGraphData] {
        for (index, extreme) in self.extremes.iter().enumerate() {
            if extreme.index >= self.start_index {
                return &self.extremes[index..];
            }
        }

        //Return empty slice
        &self.extremes[0..0]
    }

    //Tide extremes are recorded with an offset relative to the original window
    pub fn get_extreme_index_in_window(&self, extreme_index: u32) -> u32 {
        extreme_index - self.start_index
    }

    pub fn water_mark(&self) -> &WaterMarkData {
        &self.water_mark
    }
}

#[derive(Copy, Clone)]
pub struct WaterMarkData {
    pub high_water: f32,
    pub low_water: f32,
    pub current_water: f32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TideResponse {
    pub station: String,
    pub heights: Vec<TideHeightData>,
    pub extremes: Vec<TideExtremesData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TideHeightData {
    dt: u32,
    #[serde(with = "my_date_format")]
    date: DateTime<Utc>,
    pub height: f32,
}

// impl TideHeightData {
//     pub fn nil() -> TideHeightData {
//         TideHeightData {
//             height: 0_f32,
//             date: "".to_string(),
//             dt: 0_u32,
//         }
//     }
// }

pub struct TideExtremeGraphData {
    index: u32,
    date: DateTime<Utc>,
}

impl TideExtremeGraphData {
    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn date(&self) -> DateTime<Utc> {
        self.date
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TideExtremesData {
    dt: u32,
    #[serde(with = "my_date_format")]
    date: DateTime<Utc>,
    height: f32,
    #[serde(rename = "type")]
    extreme_type: String,
}

impl TideResponse {
    pub fn nil() -> TideResponse {
        TideResponse {
            station: "".to_string(),
            heights: vec![],
            extremes: vec![],
        }
    }

    pub fn new() -> TideResponse {
        // let data = r#"
        // {
        //     "station" : "Exmouth Dock",
        //     "heights": [
        //         {
        //             "dt": 1599559200,
        //             "date": "2020-09-08T10:00+0000",
        //             "height": 1.285
        //         },
        //         {
        //             "dt": 1599562800,
        //             "date": "2020-09-08T11:00+0000",
        //             "height": 1.004
        //         },
        //         {
        //             "dt": 1599566400,
        //             "date": "2020-09-08T12:00+0000",
        //             "height": 0.369
        //         }
        //     ],
        //     "extremes": [
        //         {
        //             "dt": 1599577628,
        //             "date": "2020-09-08T15:07+0000",
        //             "height": -1.396,
        //             "type": "Low"
        //         },
        //         {
        //             "dt": 1599602145,
        //             "date": "2020-09-08T21:55+0000",
        //             "height": 1.274,
        //             "type": "High"
        //         }
        //     ]
        // }"#;

        let data = match TideResponse::load_json_from_disk() {
            Ok(json) => json,
            Err(e) => {
                println!(
                    "Could not load Json from disk. Returning Empty response. Err {}",
                    e
                );
                return TideResponse::nil();
            }
        };

        let response: TideResponse = match serde_json::from_str::<TideResponse>(&data) {
            Ok(tide_response) => tide_response,
            Err(e) => {
                println!("Json parsing failed. Returning empty response. Err: {}", e);
                TideResponse::nil()
            }
        };

        response
    }

    fn load_json_from_disk() -> std::io::Result<String> {
        fs::read_to_string("resources/tides.json")
    }
}

fn find_abs_diff(a: u32, b: u32) -> u32 {
    if a > b {
        a - b
    } else {
        b - a
    }
}

//https://serde.rs/custom-date-format.html
mod my_date_format {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%dT%H:%M%z"; //2020-09-08T10:00+0000

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}
