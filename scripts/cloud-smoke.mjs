const base = (process.env.KEYHAN_CLOUD_URL || "http://localhost:3000").replace(/\/$/, "");
const token = process.env.KEYHAN_CLOUD_ACCESS_TOKEN;
const mutate = process.env.KEYHAN_CLOUD_MUTATION_TEST === "1";

async function request(path, options = {}) {
  const headers = { "content-type": "application/json", ...(options.headers || {}) };
  if (token) headers.authorization = `Bearer ${token}`;
  return fetch(`${base}${path}`, { ...options, headers, signal: AbortSignal.timeout(10000) });
}

const parsed = new URL(base);
if (parsed.protocol !== "https:" && !["localhost", "127.0.0.1"].includes(parsed.hostname)) {
  throw new Error("Cloud smoke tests require HTTPS outside localhost");
}

const unauthenticated = await fetch(`${base}/audiomaster/sync`, { signal: AbortSignal.timeout(10000) });
if (![401, 403].includes(unauthenticated.status)) throw new Error(`Unauthenticated sync returned ${unauthenticated.status}`);

try {
  await fetch("http://127.0.0.1:9/audiomaster/sync", { signal: AbortSignal.timeout(1000) });
  throw new Error("Offline behavior unexpectedly succeeded");
} catch (error) {
  if (error.message === "Offline behavior unexpectedly succeeded") throw error;
}

if (!token) {
  console.log("Unauthenticated and offline cloud checks passed; set KEYHAN_CLOUD_ACCESS_TOKEN for authenticated checks.");
  process.exit(0);
}

const currentResponse = await request("/audiomaster/sync");
if (!currentResponse.ok) throw new Error(`Authenticated sync returned ${currentResponse.status}`);
const current = await currentResponse.json();
if (!Number.isSafeInteger(current.revision) || !Array.isArray(current.presets)) throw new Error("Invalid sync document contract");

if (mutate) {
  const marker = { ...(current.settings || {}), schema_version: 1 };
  const update = await request("/audiomaster/sync", {
    method: "PUT",
    body: JSON.stringify({ base_revision: current.revision, settings: marker, presets: current.presets }),
  });
  if (!update.ok) throw new Error(`Staging mutation returned ${update.status}`);
  const updated = await update.json();
  const conflict = await request("/audiomaster/sync", {
    method: "PUT",
    body: JSON.stringify({ base_revision: current.revision, settings: marker, presets: current.presets }),
  });
  if (conflict.status !== 409) throw new Error(`Stale revision returned ${conflict.status}, expected 409`);
  const forbidden = await request("/audiomaster/sync", {
    method: "PUT",
    body: JSON.stringify({ base_revision: updated.revision, settings: { input_path: "/secret.wav" }, presets: [] }),
  });
  if (forbidden.status !== 400) throw new Error(`Forbidden path returned ${forbidden.status}, expected 400`);
  const restore = await request("/audiomaster/sync", {
    method: "PUT",
    body: JSON.stringify({ base_revision: updated.revision, settings: current.settings, presets: current.presets }),
  });
  if (!restore.ok) throw new Error(`Could not restore staging settings: ${restore.status}`);
}

console.log("KeyhanStudio Cloud smoke checks passed.");
