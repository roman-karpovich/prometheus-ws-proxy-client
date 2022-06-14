use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use serde::{de, Deserialize};


fn default_empty_string() -> String { "".to_string() }

fn default_false() -> bool { false }

fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: de::Deserializer<'de>,
{
    let v: bool = de::Deserialize::deserialize(deserializer)?;
    return Ok(v);
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub instance: String,
    pub target: String,
    pub resources: Vec<Resource>,
    #[serde(deserialize_with = "deserialize_bool", default = "default_false")]
    pub cf_access_enabled: bool,
    #[serde(default = "default_empty_string")]
    pub cf_access_key: String,
    #[serde(default = "default_empty_string")]
    pub cf_access_secret: String,
}

impl Config {
    pub fn get_resource_map(&self) -> HashMap<&String, &String> {
        let mut result = HashMap::new();
        for resource in &self.resources[..] {
            result.insert(&resource.name, &resource.url);
        }
        result
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let c = serde_json::from_reader(reader)?;
        Ok(c)
    }
}

// todo!(use name as key instead of strict naming)
#[derive(Deserialize, Debug, Clone)]
pub struct Resource {
    pub name: String,
    pub url: String,
}
