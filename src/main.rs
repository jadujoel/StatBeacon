use serde::{Deserialize, Serialize};
use std::fs;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct CliArgs {
    config: String,
}

impl Default for CliArgs {
    fn default() -> Self {
        CliArgs {
            config: "StatBeacon.toml".to_string(),
        }
    }
}

fn read_cli_args() -> CliArgs {
    let args: Vec<String> = env::args().skip(1).collect();
    let string = args.join(" ");
    if string.contains("--config") {
        let config = string.split("--config").collect::<Vec<&str>>()[1].trim();
        return CliArgs {
            config: config.to_string(),
        };
    };
    if string.contains("-c") {
        let config = string.split("-c").collect::<Vec<&str>>()[1].trim();
        return CliArgs {
            config: config.to_string(),
        };
    };
    CliArgs::default()

}

/// Configuration struct
#[derive(Deserialize, Debug, Clone)]
struct Config {
    name: String,
    interval_seconds: u64,
    target_stat_url: String,
    target_alert_url: String,
    cpu_alert_threshold: f32,
    memory_alert_threshold: f32,
    temperature_alert_threshold: f32,
}

fn read_config(cli_args: &CliArgs) -> Config {
    let config_content = fs::read_to_string(&cli_args.config)
        .unwrap_or_else(|_| panic!("Failed to read configuration file: {}", &cli_args.config));
    toml::from_str(&config_content)
        .unwrap_or_else(|e| panic!("Failed to parse configuration file: {e}"))
}

/// Example structs for logging
#[derive(Serialize, Clone, Debug)]
struct StatsLog {
    name: String,
    level: String,
    cpu: String,
    mem: String,
    temp: String,
    time: String,
}

#[derive(Serialize, Clone, Debug)]
struct TemperatureData {
    label: String,
    temperature: f32, // Celsius
}

#[tokio::main]
async fn main() {
    let cli_args: CliArgs = read_cli_args();
    let config = read_config(&cli_args);
    println!("CLI Args: {cli_args:?}");
    println!("Configuration: {config:?}");

    let client = reqwest::Client::new();

    let mut sys = sysinfo::System::new_all();
    let mut components = sysinfo::Components::new_with_refreshed_list();

    loop {
        sys.refresh_all();
        let global_cpu_usage = sys.global_cpu_usage();
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();

        let cpu = global_cpu_usage;
        let memory = (used_memory as f32 / total_memory as f32) * 100.0;

        // Formatted percentages
        let cpu_formatted = format!("{cpu:.2}%");
        let memory_formatted = format!("{memory:.2}%");
        let now = chrono::Utc::now();
        let formatted_time = now.format("%d/%m/%Y, %H:%M:%S").to_string();

        // Collect temperature data
        let mut temperature_data = Vec::new();

        components.refresh();
        for component in &components {
            let temp_data = TemperatureData {
                label: component.label().to_string(),
                temperature: component.temperature(), // Celsius
            };
            temperature_data.push(temp_data);
        }

        let average_temperature = temperature_data
            .iter()
            .fold(0.0, |acc, x| acc + x.temperature)
            / temperature_data.len() as f32;
        let temperature_formatted = format!("{average_temperature:.2}°C");

        let stats = StatsLog {
            name: config.name.clone(),
            level: "info".to_string(),
            cpu: cpu_formatted.clone(),
            mem: memory_formatted.clone(),
            temp: temperature_formatted.clone(),
            time: formatted_time.clone(),
        };

        // Post stats to the server
        if let Ok(response) = client
            .post(&config.target_stat_url)
            .json(&stats)
            .send()
            .await
        {
            if response.status().is_success() {
                // Data posted successfully
            } else {
                eprintln!("Error posting data");
            }
        }

        // Check for alerts based on thresholds
        if cpu > config.cpu_alert_threshold
            || memory > config.memory_alert_threshold
            || average_temperature > config.temperature_alert_threshold
        {
            println!(
                "Alerting CPU: {cpu_formatted}%, Memory: {memory_formatted}% and Temperature: {temperature_formatted}"
            );
            let alert = StatsLog {
                name: config.name.clone(),
                level: "warn".to_string(),
                cpu: cpu_formatted.clone(),
                mem: memory_formatted.clone(),
                temp: temperature_formatted.clone(),
                time: formatted_time.clone(),
            };
            if let Ok(response) = client
                .post(&config.target_alert_url)
                .json(&alert)
                .send()
                .await
            {
                if response.status().is_success() {
                    // Alert posted successfully
                } else {
                    eprintln!("Error posting alert data");
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(config.interval_seconds)).await;
    }
}
