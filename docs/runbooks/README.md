# Operational Runbooks

This directory contains operational runbooks for managing MCP WASM Edge Gateway in production environments.

## Available Runbooks

### Incident Response
- [Service Outage](service-outage.md) - Handle complete service failures
- [High Error Rate](high-error-rate.md) - Diagnose and fix error spikes
- [Performance Degradation](performance-degradation.md) - Address slow response times
- [Memory Issues](memory-issues.md) - Handle memory leaks and exhaustion
- [Security Incident](security-incident.md) - Respond to security breaches

### Maintenance Procedures
- [Routine Maintenance](routine-maintenance.md) - Regular maintenance tasks
- [Software Updates](software-updates.md) - Update gateway software
- [Certificate Renewal](certificate-renewal.md) - Renew TLS certificates
- [Log Rotation](log-rotation.md) - Manage log file sizes
- [Database Cleanup](database-cleanup.md) - Clean up old data

### Recovery Procedures
- [Disaster Recovery](disaster-recovery.md) - Recover from major failures
- [Data Recovery](data-recovery.md) - Restore lost or corrupted data
- [Configuration Recovery](config-recovery.md) - Restore configurations
- [Rollback Procedures](rollback.md) - Roll back failed deployments

### Monitoring and Alerting
- [Alert Response](alert-response.md) - Respond to monitoring alerts
- [Metrics Investigation](metrics-investigation.md) - Investigate metric anomalies
- [Log Analysis](log-analysis.md) - Analyze logs for issues
- [Health Check Failures](health-check-failures.md) - Handle health check issues

## Runbook Structure

Each runbook follows this structure:

### 1. Summary
- **Symptoms**: What you observe
- **Impact**: Effect on users/system
- **Urgency**: How quickly to respond

### 2. Initial Response
- **Immediate Actions**: First steps to take
- **Safety Checks**: Ensure no further damage
- **Escalation**: When to escalate

### 3. Investigation
- **Diagnostics**: How to identify the root cause
- **Tools**: Commands and tools to use
- **Common Causes**: Typical reasons for this issue

### 4. Resolution
- **Fixes**: Step-by-step resolution
- **Verification**: How to confirm the fix
- **Monitoring**: What to watch after fixing

### 5. Prevention
- **Root Cause**: Why this happened
- **Prevention**: How to prevent recurrence
- **Improvements**: Process or system improvements

## Emergency Contacts

### On-Call Engineering
- **Primary**: +1-555-0123 (John Smith)
- **Secondary**: +1-555-0456 (Jane Doe)
- **Escalation**: +1-555-0789 (Engineering Manager)

### External Support
- **Cloud Provider**: [Provider Support Portal]
- **Hardware Vendor**: [Vendor Support]
- **Security Team**: security@terragon.ai

## Escalation Matrix

| Severity | Response Time | Escalation |
|----------|---------------|------------|
| Critical | 15 minutes | Immediate |
| High | 1 hour | 2 hours |
| Medium | 4 hours | 24 hours |
| Low | 24 hours | 1 week |

## Tools and Resources

### Monitoring
- **Grafana**: https://monitoring.your-domain.com
- **Prometheus**: https://prometheus.your-domain.com
- **Logs**: `kubectl logs -f deployment/mcp-gateway`

### Documentation
- **API Docs**: https://docs.your-domain.com
- **Architecture**: [System Architecture](../ARCHITECTURE.md)
- **Configuration**: [Configuration Guide](../guides/configuration.md)

### Commands Reference

```bash
# Check service status
systemctl status mcp-gateway

# View recent logs
journalctl -u mcp-gateway -f

# Check resource usage
top -p $(pgrep mcp-gateway)

# Test connectivity
curl -f http://localhost:8080/health
```

## Contributing

To add or update runbooks:

1. Follow the standard runbook structure
2. Test all commands and procedures
3. Include real examples and outputs
4. Review with the operations team
5. Submit a pull request

## Training

New team members should:

1. Read all runbooks
2. Shadow experienced operators
3. Practice on staging environment
4. Participate in incident reviews
5. Complete runbook exercises

---

*Last Updated: 2025-01-27*
*Next Review: 2025-04-27*