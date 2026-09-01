import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";

const state = reactive({
  loading: false,
  signedIn: false,
  user: null,
  userCode: "",
  verificationUri: "",
  loginState: "idle",
  revision: 0,
  earlyAccess: false,
  error: null,
});

let pollGeneration = 0;

async function refreshStatus() {
  state.loading = true;
  state.error = null;
  try {
    const result = await invoke("cloud_status");
    state.signedIn = result.signed_in;
    state.user = result.user;
    return result;
  } catch (error) {
    state.error = String(error);
    return null;
  } finally {
    state.loading = false;
  }
}

async function beginLogin() {
  const generation = ++pollGeneration;
  state.loading = true;
  state.error = null;
  state.loginState = "starting";
  try {
    const authorization = await invoke("cloud_begin_login");
    state.userCode = authorization.user_code;
    state.verificationUri = authorization.verification_uri;
    state.loginState = "pending";
    const verificationUrl = `${authorization.verification_uri}?code=${encodeURIComponent(authorization.user_code)}`;
    await open(verificationUrl);

    const intervalMs = Math.max(authorization.interval || 5, 2) * 1000;
    const deadline = Date.now() + authorization.expires_in * 1000;
    while (generation === pollGeneration && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
      const result = await invoke("cloud_poll_login", { deviceCode: authorization.device_code });
      if (result.state === "authorized") {
        state.signedIn = true;
        state.user = result.user;
        state.loginState = "authorized";
        return result;
      }
      if (result.state !== "authorization_pending") {
        throw new Error(result.state.replaceAll("_", " "));
      }
    }
    if (generation === pollGeneration) throw new Error("Sign-in code expired");
  } catch (error) {
    if (generation === pollGeneration) {
      state.error = String(error);
      state.loginState = "error";
    }
    throw error;
  } finally {
    if (generation === pollGeneration) state.loading = false;
  }
}

async function logout() {
  ++pollGeneration;
  await invoke("cloud_logout");
  state.signedIn = false;
  state.user = null;
  state.userCode = "";
  state.loginState = "idle";
  state.revision = 0;
}

async function pullSync() {
  const document = await invoke("cloud_pull_sync");
  state.revision = document.revision;
  state.earlyAccess = document.early_access;
  return document;
}

async function pushSync(settings, presets = []) {
  const document = await invoke("cloud_push_sync", {
    baseRevision: state.revision,
    settings,
    presets,
  });
  state.revision = document.revision;
  state.earlyAccess = document.early_access;
  return document;
}

async function submitFeedback(category, message, diagnosticsOptIn) {
  await invoke("cloud_submit_feedback", { category, message, diagnosticsOptIn });
}

async function setEarlyAccess(enabled) {
  const document = await invoke("cloud_set_early_access", { enabled });
  state.revision = document.revision;
  state.earlyAccess = document.early_access;
  return document;
}

export function useCloud() {
  return {
    state,
    refreshStatus,
    beginLogin,
    logout,
    pullSync,
    pushSync,
    submitFeedback,
    setEarlyAccess,
  };
}
