# AudioMaster privacy policy

Audio processing runs locally. Audio bytes, waveforms, filenames, and local paths are not sent to KeyhanStudio Cloud. When a user explicitly selects a remote AI advisor, only the numeric analysis report and requested mastering options are sent to that configured provider; the deterministic DSP render remains local.

KeyhanStudio Cloud stores account identity, versioned application preferences, user presets, early-access enrollment, and feedback submitted by the user. Authentication refresh tokens and remote-provider API keys are stored in the operating-system credential vault. Access tokens remain in process memory.

Diagnostic logs are local and redact source paths. Diagnostics are exported only when the user requests a bundle. Crash telemetry is disabled unless a deployment supplies a Sentry DSN and the user has granted telemetry consent. Audio is never attached to telemetry.

Users may sign out to remove the local KeyhanStudio refresh token. Account data export or deletion requests are handled through KeyhanStudio support. Production operations must document the current support address and statutory response period before public release.

Cloud sync records are retained while an account is active and removed within 30 days of a verified account-deletion request. Feedback is retained for up to 24 months unless legal or security obligations require otherwise. Operational logs are retained for at most 30 days.

Policy owner: KeyhanStudio. Last revised: 2026-09-01.
