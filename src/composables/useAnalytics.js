import { invoke } from "@tauri-apps/api/core";

// Analytics state - all local, opt-in only
const state = {
  enabled: false,
  events: [],
  sessionStart: Date.now(),
};

// Generate a random session ID (no PII)
const sessionId = crypto.randomUUID();

/**
 * Initialize analytics with user consent.
 */
export function initAnalytics(enabled = false) {
  state.enabled = enabled;
  if (enabled) {
    trackEvent("session_start", { timestamp: Date.now() });
  }
}

/**
 * Track an analytics event (local-first, no network calls).
 */
export function trackEvent(event, data = {}) {
  if (!state.enabled) return;

  const entry = {
    event,
    data,
    sessionId,
    timestamp: Date.now(),
  };

  state.events.push(entry);

  // Log for development
  console.debug(`[Analytics] ${event}`, data);
}

/**
 * Track a processing event.
 */
export function trackProcessing(type, backend, durationMs, success = true) {
  trackEvent("processing", {
    type,
    backend,
    duration_ms: durationMs,
    success,
  });
}

/**
 * Track a feature usage event.
 */
export function trackFeature(feature, detail = "") {
  trackEvent("feature_used", { feature, detail });
}

/**
 * Track an error event.
 */
export function trackError(code, message, context = {}) {
  trackEvent("error", {
    code,
    message: message?.substring(0, 200), // Truncate long messages
    ...context,
  });
}

/**
 * Get all collected events (for export/diagnostics).
 */
export function getAnalyticsEvents() {
  return [...state.events];
}

/**
 * Get analytics summary.
 */
export function getAnalyticsSummary() {
  const events = state.events;
  const processing = events.filter((e) => e.event === "processing");
  const errors = events.filter((e) => e.event === "error");

  return {
    session_duration_ms: Date.now() - state.sessionStart,
    total_events: events.length,
    processing_count: processing.length,
    processing_success_rate:
      processing.length > 0
        ? processing.filter((e) => e.data.success).length / processing.length
        : 0,
    backend_usage: processing.reduce((acc, e) => {
      const backend = e.data.backend || "unknown";
      acc[backend] = (acc[backend] || 0) + 1;
      return acc;
    }, {}),
    error_count: errors.length,
    error_codes: errors.reduce((acc, e) => {
      const code = e.data.code || "unknown";
      acc[code] = (acc[code] || 0) + 1;
      return acc;
    }, {}),
  };
}

/**
 * Clear all analytics data.
 */
export function clearAnalytics() {
  state.events = [];
  state.sessionStart = Date.now();
}
