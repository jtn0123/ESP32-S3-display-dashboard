# Button Responsiveness Test Plan

## Test Overview
This test evaluates the impact of the 20ms button polling optimization on user experience.

## Metrics Being Tracked
1. **Poll Latency**: Time to detect button state change
2. **UI Response Time**: Time to update display after detection
3. **Total Response Time**: Combined poll + UI update time
4. **Event Rate**: Button events per second
5. **Max Response Time**: Worst-case latency

## Test Procedure

### 1. Flash the Test Build
```bash
./scripts/flash.sh
```

### 2. Monitor Serial Output
```bash
espflash monitor
```

### 3. Button Test Sequence
Perform the following button tests:

#### A. Single Press Test
- Press Button 1 (GPIO0) 10 times with ~1 second intervals
- Press Button 2 (GPIO14) 10 times with ~1 second intervals
- Look for [BUTTON_TEST] logs showing response times

#### B. Rapid Press Test
- Press Button 1 as fast as possible for 10 seconds
- Check if any presses are missed
- Look for [BUTTON_TEST_SUMMARY] showing events/sec

#### C. Long Press Test
- Hold Button 1 for 2+ seconds
- Verify long press detection works correctly
- Check timing accuracy

#### D. Simultaneous Press Test
- Press both buttons at the same time
- Verify both are detected
- Check for any interference

## Expected Results

### Good Performance (20ms polling):
- Average response time: 10-30ms
- Max response time: <40ms
- No missed button presses
- 20-50 events/second capability

### Issues to Watch For:
- Response times >50ms (feels sluggish)
- Missed button presses during rapid pressing
- Long press detection delays
- Any UI freezing or stuttering

## Log Output Examples

### Individual Event:
```
[BUTTON_TEST] Button event detected: Button1Click, Poll latency: 0.15ms, Time since last check: 20.05ms
[BUTTON_TEST] UI response time: 2.34ms, Total response time: 2.49ms
```

### Summary (every 10 events):
```
[BUTTON_TEST_SUMMARY] After 10 events in 8.5s: Avg response: 2.8ms, Max: 4.2ms, Events/sec: 1.2
```

## Optimization Impact

### Before (continuous polling):
- Button polling: 19,000+ times/second
- High CPU usage on Core 0
- Potential for immediate response

### After (20ms polling):
- Button polling: 50 times/second
- 99.7% reduction in polling overhead
- Maximum 20ms added latency (average 10ms)

## Human Perception Threshold
- <50ms: Feels instant
- 50-100ms: Noticeable but acceptable
- >100ms: Feels sluggish

Our 20ms polling interval keeps us well within the "instant" range.