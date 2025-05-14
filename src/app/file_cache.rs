use serde_json::from_reader;
use tracing::{debug, info};

use std::io::BufReader;

use ureq::get;

use std::collections::HashMap;

use serde_json::to_writer;

use std::io::BufWriter;

use std::fs::File;

use crate::structs::{MetaData, TLGroundStation, TLSatellite};
use crate::utils::get_data_dir;

use serde_json::to_string;

use color_eyre::Result;

pub fn cache_gs(data: Vec<TLGroundStation>) -> Result<()> {
    let mut cached_data = get_gs_cache()?;
    for i in data {
        cached_data.insert(i.station.name.clone(), to_string(&i)?);
    }
    let mut gs_file = get_data_dir();
    gs_file.push("gs.json");
    let file = File::create(gs_file)?;
    let writer = BufWriter::new(file);
    info!("Writing TLE cache: {:?}", &cached_data);
    to_writer(writer, &cached_data)?;
    Ok(())
}

pub fn cache_tle(data: &Vec<TLSatellite>) -> Result<()> {
    let cache_result = get_sat_cache();
    let mut cache_data;
    if cache_result.is_err() {
        cache_data = HashMap::new();
    } else {
        cache_data = cache_result.unwrap();
    }
    for i in data {
        cache_data.insert(i.satellite.get_norad_id().to_string(), to_string(&i)?);
    }
    let mut tle_file = get_data_dir();
    tle_file.push("tle.json");
    let file = File::create(tle_file)?;
    let writer = BufWriter::new(file);
    info!("Writing TLE cache: {:?}", &cache_data);
    to_writer(writer, &cache_data)?;
    Ok(())
}

pub fn get_sup_data_spacetrack(norad_id: &str) -> Result<MetaData> {
    let response = get(format!(
        "https://celestrak.org/satcat/records.php?CATNR={}",
        norad_id
    ))
    .call()?;
    let response_lose: Vec<MetaData> = response.into_body().read_json()?;
    debug!("{:?}", response_lose);
    Ok(response_lose[0].clone())
}

pub fn get_tle_spacetrack(norad_id: u64) -> Result<String> {
    let response = get(format!(
        "https://celestrak.org/NORAD/elements/gp.php?CATNR={}&FORMAT=TLE",
        norad_id
    ))
    .call()?;
    Ok(response.into_body().read_to_string()?)
}

pub fn get_sat_cache() -> Result<HashMap<String, String>> {
    get_cache_file("tle.json")
}

pub fn get_gs_cache() -> Result<HashMap<String, String>> {
    get_cache_file("gs.json")
}

fn get_cache_file(filename: &str) -> Result<HashMap<String, String>> {
    let mut data_dir = get_data_dir();
    data_dir.push(filename);
    if data_dir.try_exists()? {
        let file = File::open(data_dir)?;
        let reader = BufReader::new(file);
        Ok(from_reader(reader)?)
    } else {
        let file = File::create_new(data_dir)?;
        let writer = BufWriter::new(file);
        to_writer(writer, &HashMap::<String, String>::new())?;
        Ok(HashMap::new())
    }
}
