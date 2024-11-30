# StatBeacon

## Usage

```bash
StatBeacon --config=/path/to/StatBeacon.toml
```

## Example Config File

```toml
# StatBeacon.toml
name = "My Application"
interval_seconds = 4
target_stat_url = "https://example.com/api/stats"
target_alert_url = "http://example/api/alerts"
cpu_alert_threshold = 60
memory_alert_threshold = 80
temperature_alert_threshold = 70
```
