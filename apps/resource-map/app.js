"use strict";

// This alpha uses zero browser dependencies. The shell/runtime does NOT depend on Chromium.
// The UI is static and can later be migrated to React/Helm without changing the API.
const byId = (id) => document.getElementById(id);
const params = new URLSearchParams(window.location.hash.slice(1));
const token = params.get("token");
let lastSnapshot = null;
let inFlight = false;

function setText(id, value) { byId(id).textContent = value; }
function percentage(value) { return Math.max(0, Math.min(100, value)); }
function gib(bytes) { return `${(Number(bytes) / (1024 ** 3)).toFixed(1)} GiB`; }
function setMeter(prefix, ratio) {
  const valid = Number.isFinite(ratio);
  setText(`${prefix}-value`, valid ? String(Math.round(percentage(ratio))) : "—");
  byId(`${prefix}-bar`).style.width = valid ? `${percentage(ratio)}%` : "0%";
}
function readableUptime(value) {
  if (!Number.isFinite(value)) return "—";
  const days = Math.floor(value / 86400);
  const hours = Math.floor(value % 86400 / 3600);
  const mins = Math.floor(value % 3600 / 60);
  return `${days ? `${days}d ` : ""}${hours}h ${mins}m`;
}
function update(snapshot) {
  lastSnapshot = snapshot;
  setText("map-hostname", snapshot.node.name);
  setText("host-title", snapshot.node.name);
  setText("os", `${snapshot.node.operating_system}${snapshot.node.is_wsl ? " / WSL2" : ""}`);
  setText("cores", String(snapshot.cpu.logical_cores));
  setText("load", snapshot.cpu.load_one_minute?.toFixed(2) ?? "—");
  setText("uptime", readableUptime(snapshot.node.uptime_seconds));
  setText("gpu-status", "Not yet probed");
  setText("peers-value", String(snapshot.fabric.connected_peers.length));

  setMeter("cpu", snapshot.cpu.usage_percent);
  setText("cpu-hint", `${snapshot.cpu.logical_cores} logical cores · ${snapshot.cpu.usage_percent === null ? "sampling…" : "active"}`);

  const mem = snapshot.memory;
  const memUsage = mem.total_bytes > 0 ? (100 * mem.used_bytes / mem.total_bytes) : null;
  setMeter("ram", memUsage);
  setText("ram-hint", `${gib(mem.used_bytes)} / ${gib(mem.total_bytes)}`);
  setText("ram-free", gib(mem.available_bytes));

  if (snapshot.storage && snapshot.storage.total_bytes > 0) {
    const d = snapshot.storage;
    setMeter("disk", 100 * d.used_bytes / d.total_bytes);
    setText("disk-hint", `${gib(d.used_bytes)} / ${gib(d.total_bytes)}`);
  } else {
    setMeter("disk", NaN);
    setText("disk-hint", "Storage data unavailable");
  }

  const iface = snapshot.network.filter((item) => item.link_state === "up");
  setText("interfaces", iface.length ? iface.map((n) => n.name).join(", ") : "None active");
  byId("connection").className = "connection status-live";
  byId("connection").innerHTML = "<span class='light'></span>Local node online";
  byId("error").hidden = true;
  setText("refresh-time", `Updated ${new Date(snapshot.collected_at_ms).toLocaleTimeString()}`);
}

async function refresh() {
  if (inFlight || !token) return;
  inFlight = true;
  try {
    const response = await fetch("/api/v1/local", {
      headers: { Authorization: `Bearer ${token}` },
      cache: "no-store",
    });
    if (!response.ok) throw new Error(`Telemetry API returned ${response.status}`);
    const snapshot = await response.json();
    if (snapshot.schema_version !== 1) throw new Error("Unsupported local API version");
    update(snapshot);
  } catch (error) {
    byId("connection").className = "connection status-error";
    byId("connection").innerHTML = "<span class='light'></span>Disconnected";
    setText("error", `Unable to reach the local zh agent: ${error.message}`);
    byId("error").hidden = false;
  } finally {
    inFlight = false;
  }
}

byId("copy-json").addEventListener("click", async () => {
  if (!lastSnapshot) return;
  try {
    await navigator.clipboard.writeText(JSON.stringify(lastSnapshot, null, 2));
    setText("copy-json", "Copied!");
    setTimeout(() => setText("copy-json", "Copy resource JSON"), 1600);
  } catch {
    setText("copy-json", "Copy unavailable");
  }
});

if (!token) {
  byId("error").hidden = false;
  setText("error", "Missing session credential. Relaunch the resource map using zh.");
  byId("connection").className = "connection status-error";
  byId("connection").innerHTML = "<span class='light'></span>No session";
} else {
  void refresh();
  setInterval(refresh, 2000);
}
