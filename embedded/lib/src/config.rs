pub struct EnvConfig {
    pub wifi_ssid: &'static str,
    pub wifi_password: &'static str,
}

pub const ENV_CONFIG: EnvConfig = EnvConfig {
    wifi_ssid: env!("WIFI_SSID"),
    wifi_password: env!("WIFI_PASSWORD"),
};
