use envconfig::Envconfig;
use std::time::Duration;

#[derive(Envconfig, Debug, Clone)]
pub struct SearchConfig {
    #[envconfig(from = "SEARCH_PROVIDER_ENDPOINT", default = "http://localhost:9200")]
    pub endpoint: String,
    
    #[envconfig(from = "SEARCH_PROVIDER_TIMEOUT", default = "30")]
    pub timeout_secs: u64,
    
    #[envconfig(from = "SEARCH_PROVIDER_MAX_RETRIES", default = "3")]
    pub max_retries: u32,
    
    #[envconfig(from = "SEARCH_PROVIDER_LOG_LEVEL", default = "info")]
    pub log_level: String,
    
    #[envconfig(from = "ELASTIC_USERNAME")]
    pub username: Option<String>,
    
    #[envconfig(from = "ELASTIC_PASSWORD")]
    pub password: Option<String>,
    
    #[envconfig(from = "ELASTIC_CLOUD_ID")]
    pub cloud_id: Option<String>,
    
    #[envconfig(from = "ELASTIC_API_KEY")]
    pub api_key: Option<String>,
}

impl SearchConfig {
    pub fn from_env() -> Result<Self, envconfig::Error> {
        Self::init_from_env()
    }
    
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
} 