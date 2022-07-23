#[cfg(test)]
mod tests {
    use mockito::mock;
    use crate::config::Config;
    use std::collections::HashMap;

    fn get_example_resources() -> HashMap<String, String> {
        HashMap::from([
            ("node".to_string(), "https://localhost:9100".to_string()),
            ("example".to_string(), "https://example.com".to_string()),
        ])
    }

    const EXAMPLE_TARGET: &str = "http://example.com/proxy/ws/";

    #[test]
    fn test_static_instance_name() {
        let c = Config::new(
            "test".to_string(), EXAMPLE_TARGET.to_string(),
            get_example_resources(),
            false, "".to_string(), "".to_string(),
        );
        assert_eq!(c.get_instance_name(), "test");
    }

    #[test]
    #[should_panic(expected = "Unable to resolve instance name. Maybe it's not ec2 instance?")]
    fn test_ec2_instance_name_panic() {
        let _mc = mock("GET", "/latest/meta-data/instance-id")
            .with_status(502)
            .create();

        Config::new(
            "ec2".to_string(), EXAMPLE_TARGET.to_string(),
            get_example_resources(),
            false, "".to_string(), "".to_string(),
        );
    }

    #[test]
    fn test_ec2_instance_name() {
        let _mc = mock("GET", "/latest/meta-data/instance-id")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("test-ec2-instance")
            .create();

        let c = Config::new(
            "ec2".to_string(), EXAMPLE_TARGET.to_string(),
            get_example_resources(),
            false, "".to_string(), "".to_string(),
        );
        assert_eq!(c.get_instance_name(), "test-ec2-instance");
    }
}
