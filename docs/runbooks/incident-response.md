# AudioMaster — Incident Response Runbook

## Common Issues

### Backend Processing Failures

**Symptoms:** Users report "Backend processing failed" errors.

**Steps:**
1. Check Sentry for error details (filter by `backend_error` tag)
2. Verify Python environment: `python3 -c "import matchering; import pedalboard"`
3. Check if required scripts exist in `python/` directory
4. Verify input file is valid audio (not corrupted)
5. Check disk space for output files

### AI Backend Not Responding

**Symptoms:** AI mastering hangs or fails with timeout.

**Steps:**
1. Check if Ollama is running: `curl http://localhost:11434/api/tags`
2. Check if model is available: `ollama list`
3. For cloud providers, check API status pages
4. Verify API keys in config: `~/.config/mastering/config.toml`
5. Check network connectivity

### Memory Issues with Large Files

**Symptoms:** App crashes or becomes unresponsive with large audio files.

**Steps:**
1. Check file size (max supported: 500MB)
2. Check available RAM
3. Look for memory leak in logs
4. Try with smaller file to isolate issue
5. Check if issue is format-specific

### App Won't Start

**Symptoms:** Application fails to launch.

**Steps:**
1. Check crash logs in `~/Library/Logs/AudioMaster/`
2. Verify macOS version (minimum: macOS 12+)
3. Check if another instance is running
4. Try deleting config: `rm ~/.config/mastering/config.toml`
5. Check Sentry for crash reports

## Diagnostic Bundle Export

Users can export a diagnostic bundle from Settings > Export Diagnostics. This includes:
- Recent log files (last 3 days)
- System information
- Python environment details

## Escalation

For issues that can't be resolved:
1. Create a GitHub Issue with the diagnostic bundle attached
2. Include steps to reproduce
3. Tag with appropriate labels (bug, backend, ui)
