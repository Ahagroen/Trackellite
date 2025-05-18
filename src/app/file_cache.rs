#[cfg(not(target_arch = "wasm32"))]
pub mod cache {
    use crate::structs::{MetaData, TLSatellite};
    use crate::utils::native::get_data_dir;
    use tracing::{debug, info};
    use ureq::get;

    use serde_json::to_string;

    use crate::structs::TLGroundStation;

    use serde_json::to_writer;

    use std::io::BufWriter;

    use serde_json::from_reader;

    use std::io::BufReader;

    use std::fs::File;

    use std::collections::HashMap;

    use color_eyre::Result;

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
}
#[cfg(target_arch = "wasm32")]
pub mod cache {
    use crate::structs::{MetaData, TLSatellite};

    use color_eyre::eyre::eyre;
    use serde_json::from_str;
    use serde_json::to_string;
    use tracing::debug;
    use tracing::info;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_futures::spawn_local;
    use web_sys::Request;
    use web_sys::RequestInit;
    use web_sys::Response;

    use crate::structs::TLGroundStation;

    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::Mutex;

    use color_eyre::{Result, eyre::Error};
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
        info!("Writing TLE cache: {:?}", &cache_data);
        let string_data = to_string(&cache_data)?;
        put_data_in_cache("tle", &string_data)?;
        Ok(())
    }
    pub fn cache_gs(data: Vec<TLGroundStation>) -> Result<()> {
        let cache_result = get_gs_cache();
        let mut cache_data;
        if cache_result.is_err() {
            cache_data = HashMap::new();
        } else {
            cache_data = cache_result.unwrap();
        }
        for i in data {
            cache_data.insert(i.station.name.clone(), to_string(&i)?);
        }
        info!("Writing GS cache: {:?}", &cache_data);
        let string_data = to_string(&cache_data)?;
        put_data_in_cache("gs", &string_data)?;
        Ok(())
    }
    fn put_data_in_cache(key: &str, data: &str) -> Result<()> {
        let window = web_sys::window().unwrap();
        window
            .local_storage()
            .map_err(|_| eyre!("Unable to set {} in cache", key))?
            .unwrap()
            .set_item(key, data)
            .map_err(|_| eyre!("Unable to set {} in cache", key))
    }
    pub fn get_sup_data_spacetrack(norad_id: &str) -> Result<MetaData> {
        info!("Calling Celestrak: SUP data");
        let url = format!(
            "https://celestrak.org/satcat/records.php?CATNR={}",
            norad_id
        );
        let response = get_request(&url)?;
        let parsed_result: MetaData = serde_wasm_bindgen::from_value(response)
            .map_err(|_| Error::msg("Unable to deserialize metadata"))?;
        Ok(parsed_result)
    }
    fn get_request(url: &str) -> Result<JsValue> {
        let opts = RequestInit::new();
        debug!("Starting request");
        opts.set_method("GET");
        let request = Request::new_with_str_and_init(url, &opts).unwrap();
        let window = web_sys::window().unwrap();
        debug!("Got Window");
        let response_outer: Arc<Mutex<Option<Result<JsValue>>>> = Arc::new(Mutex::new(None));
        let response_in = response_outer.clone();
        let future = async move {
            let mut response_inner = response_in.lock().unwrap();
            let response_result = JsFuture::from(window.fetch_with_request(&request)).await;
            let response;
            match response_result {
                Ok(x) => response = x,
                Err(_) => {
                    response_inner.replace(Err(Error::msg("unable to complete request")));
                    return;
                }
            }
            debug!("Got Response");
            let resp: Response;
            let resp_result = response
                .dyn_into()
                .map_err(|_| Error::msg("unable to complete request"));
            match resp_result {
                Ok(x) => resp = x,
                Err(_) => {
                    response_inner.replace(Err(Error::msg("unable to complete request")));
                    return;
                }
            }
            debug!("Got response");
            let pre_json = resp.json().map_err(|_| {
                response_inner.replace(Err(Error::msg("unable to complete request")));
                return;
            });
            if pre_json.is_ok() {
                let json = JsFuture::from(pre_json.unwrap()).await.map_err(|_| {
                    response_inner.replace(Err(Error::msg("unable to complete request")));
                    return;
                });
                if json.is_ok() {
                    response_inner.replace(Ok(json.unwrap()));
                }
            };
        };
        spawn_local(future);
        loop {
            if let Ok(x) = response_outer.try_lock() {
                if let Some(y) = x.as_ref() {
                    debug!("Unwraped mutex guard");
                    match y {
                        Ok(val) => return Ok(val.clone()),
                        Err(_) => return Err(Error::msg("unable to complete request")),
                    }
                } else {
                }
            }
        }
    }
    pub fn get_tle_spacetrack(norad_id: u64) -> Result<String> {
        info!("Calling Celestrak: TLE");
        let url = format!(
            "https://celestrak.org/NORAD/elements/gp.php?CATNR={}&FORMAT=TLE",
            norad_id
        );
        let response = get_request(&url)?;
        let parsed_result: String = serde_wasm_bindgen::from_value(response)
            .map_err(|_| Error::msg("Unable to deserialize metadata"))?;
        Ok(parsed_result)
    }
    fn get_data_from_cache(key: &str) -> Result<HashMap<String, String>> {
        let window = web_sys::window().unwrap();
        let data = window
            .local_storage()
            .map_err(|_| Error::msg("Unable to get TLE from cache"))?
            .unwrap()
            .get_item(key)
            .map_err(|_| Error::msg("Unable to get TLE from cache"))?
            .unwrap_or_default();
        let result: HashMap<String, String> = from_str(&data)?;
        Ok(result)
    }
    pub fn get_sat_cache() -> Result<HashMap<String, String>> {
        get_data_from_cache("tle")
    }
    pub fn get_gs_cache() -> Result<HashMap<String, String>> {
        get_data_from_cache("gs")
    }
}
