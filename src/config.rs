use crate::utils::get_ec2_instance_name;
use serde::{de, Deserialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

fn default_empty_string() -> String {
    "".to_string()
}

fn default_false() -> bool {
    false
}

fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: de::Deserializer<'de>,
{
    let v: bool = de::Deserialize::deserialize(deserializer)?;
    Ok(v)
}

fn ec2_meta_domain() -> String {
    "http://169.254.169.254".to_string()
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    instance: String,
    pub target: String,
    pub resources: HashMap<String, String>,
    #[serde(deserialize_with = "deserialize_bool", default = "default_false")]
    pub cf_access_enabled: bool,
    #[serde(default = "default_empty_string")]
    pub cf_access_key: String,
    #[serde(default = "default_empty_string")]
    pub cf_access_secret: String,
    #[serde(default = "ec2_meta_domain")]
    ec2_meta_domain: String
}

impl Config {
    #[cfg(test)]
    pub fn new(
        instance: String,
        target: String,
        resources: HashMap<String, String>,
        cf_access_enabled: bool,
        cf_access_key: String,
        cf_access_secret: String,
        ec2_meta_domain: String,
    ) -> Config {
        let mut c = Config {
            instance: "".to_string(),
            target,
            resources,
            cf_access_enabled,
            cf_access_key,
            cf_access_secret,
            ec2_meta_domain,
        };
        c.set_instance_name(instance);
        c
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut c: Config = serde_json::from_reader(reader)?;
        c.set_instance_name(c.instance.clone());
        Ok(c)
    }

    pub fn get_instance_name(&self) -> &String {
        &self.instance
    }

    pub fn set_instance_name(&mut self, value: String) {
        let mut instance_name = value;
        if instance_name == "ec2" {
            instance_name = get_ec2_instance_name(&self.ec2_meta_domain)
        }
        self.instance = instance_name;
    }
}
