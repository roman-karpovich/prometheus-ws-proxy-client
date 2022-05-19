## Prometheus websocket proxy client
collect prometheus metrics from private networks without ports configuration / opening

### Configuration
Configuration is performed in json config. Parameters:  
`instance` - instance name to call client from server  
`target` - proxy server websocket endpoint  
`resources` - list of resources available for calling. key pairs of name and url  
`cf_access_enabled` - provide cloudflare headers if enabled   
`cf_access_key` - cloudflare access key  
`cf_access_secret` - cloudflare access secret  

### Usage
`proxy-client ./test.json`
