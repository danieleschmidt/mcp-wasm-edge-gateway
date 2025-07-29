# Incident Response Playbook

## Overview

This document outlines the incident response procedures for the MCP WASM Edge Gateway. It provides structured approaches for detecting, responding to, and recovering from production incidents.

## Incident Classification

### Severity Levels

**SEV-1 (Critical)**
- Complete service outage
- Security breach or data compromise
- Performance degradation >90%
- Impact: All users affected

**SEV-2 (High)**
- Partial service disruption
- Significant performance issues (50-90% degradation)
- Feature unavailability
- Impact: Subset of users affected

**SEV-3 (Medium)**
- Minor service issues
- Performance degradation <50%
- Non-critical feature issues
- Impact: Limited user impact

**SEV-4 (Low)**
- Cosmetic issues
- Documentation problems
- Minor configuration issues
- Impact: Minimal user impact

## Detection & Alerting

### Automated Monitoring

**Health Check Failures**
```bash
# Check service health
curl -f http://localhost:8080/health || echo "ALERT: Health check failed"

# Check specific components
curl -s http://localhost:8080/health | jq '.components[] | select(.status != "healthy")'
```

**Performance Thresholds**
- Response time >2s (P95)
- Error rate >5%
- Memory usage >80%
- CPU usage >85%
- Queue size >1000 items

**Key Metrics to Monitor**
```promql
# High error rate
rate(http_requests_total{status=~"5.."}[5m]) > 0.05

# High response time
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 2

# Memory usage
process_resident_memory_bytes / 1024 / 1024 / 1024 > 0.8

# Queue backlog
mcp_queue_size > 1000
```

### Alert Channels

**Primary Escalation**
1. On-call engineer (PagerDuty/SMS)
2. #incidents Slack channel
3. Engineering lead
4. Product owner (SEV-1/2 only)

**Communication Channels**
- Internal: #incidents
- Customer: Status page updates
- Stakeholders: Email notifications

## Response Procedures

### Initial Response (0-15 minutes)

**SEV-1/2 Incidents**
1. **Acknowledge** the alert immediately
2. **Assess** impact and scope
3. **Communicate** in #incidents channel
4. **Investigate** root cause
5. **Mitigate** if possible

**Investigation Commands**
```bash
# Check service status
systemctl status mcp-gateway

# View recent logs
journalctl -u mcp-gateway --since "10 minutes ago" -f

# Check resource usage
top -p $(pgrep mcp-gateway)
free -h
df -h

# Check network connectivity
ss -tlnp | grep :8080
curl -I http://localhost:8080/health

# Check database/storage
ls -la /var/lib/mcp/queue/
sqlite3 /var/lib/mcp/queue.db ".tables"
```

### Incident Investigation

**Log Analysis**
```bash
# Error patterns
grep -i "error\|fail\|panic" /var/log/mcp-gateway/app.log | tail -50

# Performance issues
grep "slow\|timeout\|deadline" /var/log/mcp-gateway/app.log

# Memory/resource issues
grep -E "(oom|memory|resource)" /var/log/mcp-gateway/app.log

# Security issues
grep -E "(auth|security|intrusion)" /var/log/mcp-gateway/security.log
```

**Metrics Dashboard**
- Grafana: http://monitoring.local:3000/d/mcp-gateway
- Key panels: Request rate, error rate, response time, resource usage

**Database Queries**
```sql
-- Check queue status
SELECT status, COUNT(*) FROM queue_items GROUP BY status;

-- Recent errors
SELECT timestamp, error_type, message FROM error_log 
WHERE timestamp > datetime('now', '-1 hour') 
ORDER BY timestamp DESC LIMIT 50;

-- Performance metrics
SELECT AVG(response_time_ms), MAX(response_time_ms) 
FROM request_log 
WHERE timestamp > datetime('now', '-10 minutes');
```

### Mitigation Strategies

**Service Recovery**
```bash
# Restart service
sudo systemctl restart mcp-gateway

# Graceful restart with health check
sudo systemctl stop mcp-gateway
sleep 10
sudo systemctl start mcp-gateway
sleep 5
curl -f http://localhost:8080/health

# Scale horizontally (if using containers)
docker-compose up --scale mcp-gateway=3
```

**Traffic Management**
```bash
# Enable circuit breaker
curl -X POST http://localhost:8080/admin/circuit-breaker/enable

# Reduce load
curl -X POST http://localhost:8080/admin/rate-limit -d '{"requests_per_second": 10}'

# Drain traffic gradually
nginx -s reload  # After updating upstream configuration
```

**Resource Management**
```bash
# Clear caches
curl -X POST http://localhost:8080/admin/cache/clear

# Force garbage collection
kill -USR1 $(pgrep mcp-gateway)

# Restart with increased resources
docker update --memory=2g --cpus=2.0 mcp-gateway
```

### Communication Templates

**Initial Alert**
```
🚨 INCIDENT - SEV-[1/2/3/4] - [Brief Description]

STATUS: Investigating
IMPACT: [User impact description]
SCOPE: [Affected components/users]
LEAD: @[incident-lead]

Updates: Thread 🧵
```

**Status Updates**
```
📊 UPDATE - [Timestamp]

PROGRESS: [What we've learned/tried]
CURRENT STATUS: [Service status]
NEXT STEPS: [Planned actions]
ETA: [Expected resolution time]
```

**Resolution Notice**
```
✅ RESOLVED - [Timestamp]

SOLUTION: [What fixed the issue]
ROOT CAUSE: [Brief explanation]
FOLLOW-UP: [Action items/prevention]
POST-MORTEM: [Link when available]
```

## Escalation Matrix

### Severity-Based Escalation

**SEV-1: Immediate**
- 0 min: On-call engineer
- 15 min: Engineering lead
- 30 min: CTO/VP Engineering
- 60 min: CEO (if customer-facing)

**SEV-2: Rapid**
- 0 min: On-call engineer
- 30 min: Engineering lead
- 2 hours: CTO/VP Engineering

**SEV-3/4: Standard**
- 0 min: On-call engineer
- Next business day: Engineering lead

### Contact Information

**On-Call Rotation**
- Primary: [Phone/Slack]
- Secondary: [Phone/Slack]
- Schedule: [PagerDuty/Calendar link]

**Engineering Leadership**
- Team Lead: [Contact info]
- Engineering Manager: [Contact info]
- CTO: [Contact info]

## Recovery Procedures

### Service Restoration

**Database Recovery**
```bash
# Backup current state
cp /var/lib/mcp/queue.db /var/lib/mcp/queue.db.backup

# Restore from backup
systemctl stop mcp-gateway
cp /var/lib/mcp/backups/queue-$(date +%Y%m%d).db /var/lib/mcp/queue.db
systemctl start mcp-gateway

# Verify integrity
sqlite3 /var/lib/mcp/queue.db "PRAGMA integrity_check;"
```

**Configuration Recovery**
```bash
# Restore configuration
git checkout HEAD -- config/production.toml

# Validate configuration
cargo run --bin validate-config -- config/production.toml

# Apply configuration
systemctl reload mcp-gateway
```

**Data Recovery**
```bash
# Check for data corruption
find /var/lib/mcp -name "*.corrupt" -delete

# Rebuild indices
curl -X POST http://localhost:8080/admin/rebuild-indices

# Resync with upstream
curl -X POST http://localhost:8080/admin/sync/force
```

### Performance Recovery

**Memory Issues**
```bash
# Check for memory leaks
valgrind --tool=memcheck --leak-check=full ./target/release/mcp-gateway

# Monitor memory usage
watch -n 1 'ps aux | grep mcp-gateway | grep -v grep'

# Implement memory limits
systemd-run --slice=mcp --property=MemoryMax=1G mcp-gateway
```

**CPU Issues**
```bash
# Profile CPU usage
perf record -g ./target/release/mcp-gateway
perf report

# Check for CPU contention
htop -u mcp-gateway

# Adjust CPU limits
docker update --cpus=1.5 mcp-gateway
```

## Post-Incident Activities

### Immediate Actions (0-24 hours)

1. **Service Stabilization**
   - Monitor key metrics for 2x normal duration
   - Verify all alerts are resolved
   - Confirm customer impact is mitigated

2. **Initial Assessment**
   - Document timeline of events
   - Collect relevant logs and metrics
   - Identify contributing factors

3. **Customer Communication**
   - Update status page
   - Send resolution notification
   - Prepare customer-facing summary

### Post-Mortem Process

**Timeline: 3-5 business days after resolution**

**Required Attendees**
- Incident lead
- On-call engineer(s)
- Engineering manager
- Product owner (if customer impact)

**Agenda Template**
1. Incident timeline
2. Root cause analysis
3. Impact assessment
4. Response effectiveness
5. Lessons learned
6. Action items

**Post-Mortem Template**
```markdown
# Post-Mortem: [Incident Title]

## Summary
- **Date**: [Date/Time]
- **Duration**: [Total duration]
- **Severity**: [SEV level]
- **Impact**: [Customer/business impact]

## Timeline
- [Time]: [Event description]
- [Time]: [Response action]
- [Time]: [Resolution]

## Root Cause
[Detailed root cause analysis]

## Contributing Factors
- [Factor 1]
- [Factor 2]

## What Went Well
- [Positive aspects of response]

## What Could Be Improved
- [Areas for improvement]

## Action Items
- [ ] [Action item] - Owner: [Name] - Due: [Date]
- [ ] [Action item] - Owner: [Name] - Due: [Date]

## Lessons Learned
[Key takeaways for future incidents]
```

### Prevention Measures

**Technical Improvements**
- Monitoring enhancements
- Performance optimizations
- Reliability improvements
- Security hardening

**Process Improvements**
- Runbook updates
- Training programs
- Alert tuning
- Documentation updates

**Testing Enhancements**
- Chaos engineering
- Load testing
- Disaster recovery drills
- Security penetration testing

## References

### Runbooks
- [Service Deployment](./DEPLOYMENT.md)
- [Monitoring Setup](../monitoring/README.md)
- [Security Procedures](./SECURITY.md)
- [Performance Tuning](./PERFORMANCE.md)

### Tools
- **Monitoring**: Grafana, Prometheus
- **Logging**: Journald, Fluentd
- **Alerting**: PagerDuty, Slack
- **Communication**: Slack, Email, Status page

### Emergency Contacts
- Emergency Hotline: [Number]
- Security Team: [Email]
- Infrastructure Team: [Email]
- Customer Success: [Email]

---

*This playbook should be reviewed quarterly and updated after each major incident.*