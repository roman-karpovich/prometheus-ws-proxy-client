use std::time::Duration;

const EC2_META_DOMAIN: &str = "http://169.254.169.254";

#[cfg(test)]
use mockito::server_url;

#[allow(unused)]
fn get_domain(domain: &str) -> String {
    #[cfg(test)]
    return server_url();

    #[cfg(not(test))]
    return domain.to_string();
}

pub fn get_ec2_instance_name() -> String {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5)).build().unwrap();

    let url = format!(
        "{}{}",
        get_domain(EC2_META_DOMAIN),
        "/latest/meta-data/instance-id"
    );
    let response = client.get(url).send();
    if response.is_err() {
        panic!("Unable to resolve instance name. Maybe it's not ec2 instance?");
    }
    let response_obj = response.unwrap();
    if response_obj.status() != 200 {
        panic!("Unable to resolve instance name. Maybe it's not ec2 instance?");
    }
    response_obj.text().unwrap()
}
