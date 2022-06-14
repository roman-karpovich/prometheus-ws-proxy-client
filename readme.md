## Prometheus websocket proxy client
collect prometheus metrics from private networks without ports configuration / opening

### Configuration
Configuration is performed in json config. Parameters:  
`instance` - instance name to call client from server  
`target` - proxy server websocket endpoint  
`resources` - hashmap of resources available for server. `{"node": "http://localhost:9100/exporter/", "example": "http://example.com"}`  
`cf_access_enabled` - provide cloudflare headers if enabled. important when your prometheus websocket server is protected with cloudflare access. (https://blog.cloudflare.com/give-your-automated-services-credentials-with-access-service-tokens/)   
`cf_access_key` - cloudflare access key  
`cf_access_secret` - cloudflare access secret  

### Usage
`proxy-client ./test.json`
