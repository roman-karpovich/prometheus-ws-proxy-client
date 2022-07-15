use std::time::Duration;

const EC2_INSTANCE_ID_URL: &str = "http://169.254.169.254/latest/meta-data/instance-id";

pub fn get_ec2_instance_name() -> String {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5)).build().unwrap();
    let response = client.get(EC2_INSTANCE_ID_URL).send();
    if response.is_err() {
        panic!("Unable to resolve instance name. Maybe it's not ec2 instance?");
    }
    response.unwrap().text().unwrap()
}
