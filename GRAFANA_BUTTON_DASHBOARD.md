# Grafana Dashboard for Button Responsiveness

## Button Metrics Available

The ESP32 now exports the following button metrics via Prometheus:

### Metrics Exported
- `esp32_button_avg_response_milliseconds` - Average button response time (gauge)
- `esp32_button_max_response_milliseconds` - Maximum button response time (gauge)
- `esp32_button_events_total` - Total button events processed (counter)
- `esp32_button_events_per_second` - Button events per second (gauge)

## Setting Up in Grafana

### 1. Access Metrics Endpoint
The metrics are available at: `http://<ESP32-IP>/metrics`

### 2. Create Dashboard Panels

#### Panel 1: Response Time Graph
```
Query: esp32_button_avg_response_milliseconds
Legend: Average Response Time
```

#### Panel 2: Max Response Time
```
Query: esp32_button_max_response_milliseconds
Legend: Max Response Time
```

#### Panel 3: Events Per Second
```
Query: esp32_button_events_per_second
Legend: Button Events/sec
```

#### Panel 4: Total Events Counter
```
Query: rate(esp32_button_events_total[1m]) * 60
Legend: Events per minute
```

### 3. Alert Rules

Create alerts for degraded performance:

#### Slow Response Alert
```
Alert when: avg(esp32_button_avg_response_milliseconds) > 50
For: 5m
Summary: Button response time exceeding 50ms
```

#### Max Response Alert
```
Alert when: esp32_button_max_response_milliseconds > 100
For: 1m
Summary: Button response spike detected
```

## Example Grafana JSON Panel

```json
{
  "title": "Button Response Time",
  "type": "graph",
  "targets": [
    {
      "expr": "esp32_button_avg_response_milliseconds",
      "legendFormat": "Average",
      "refId": "A"
    },
    {
      "expr": "esp32_button_max_response_milliseconds",
      "legendFormat": "Maximum",
      "refId": "B"
    }
  ],
  "yaxes": [
    {
      "format": "ms",
      "label": "Response Time"
    }
  ]
}
```

## Testing the Metrics

1. Flash the device: `./scripts/flash.sh`
2. Verify metrics endpoint: `curl http://<ESP32-IP>/metrics | grep button`
3. Press buttons and watch metrics update
4. Import into Grafana for continuous monitoring

## Expected Values

- **Good**: Average < 30ms, Max < 50ms
- **Acceptable**: Average < 50ms, Max < 100ms
- **Poor**: Average > 50ms or Max > 100ms

The 20ms polling optimization should keep response times well within the "Good" range.