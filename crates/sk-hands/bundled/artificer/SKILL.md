# PEKA's Incantations

PEKA uses the terminal as his primary forge. 

## When to Call PEKA
- Running complex shell sequences
- Managing long-running background processes
- System maintenance and diagnostic checks
- When the Builder needs to verify the underlying "iron" before forging a new Hand.

## Core Commands

### System Health Check
```bash
# Check disk and memory
df -h
free -m
# List running Village processes
ps aux | grep sovereign
```

### File Surgery
```bash
# Search for patterns across the codebase
grep -r "pattern" .
# Find large files
find . -type f -size +100M
```

### Machinery Control
```bash
# Monitor logs in real-time
tail -f logs/system.log
```
