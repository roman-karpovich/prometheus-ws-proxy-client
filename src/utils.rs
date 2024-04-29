use std::time::Duration;

pub fn get_ec2_instance_name(domain: &String) -> String {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let url = format!(
        "{}{}",
        domain,
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
