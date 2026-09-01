import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const destination = process.argv[2] || "THIRD_PARTY_NOTICES.md";
const cargo = JSON.parse(execFileSync("cargo", ["metadata", "--format-version", "1"], {
  encoding: "utf8",
  maxBuffer: 128 * 1024 * 1024,
  stdio: ["ignore", "pipe", "ignore"],
}));
const rust = cargo.packages
  .filter((pkg) => !pkg.source?.startsWith("path+file:"))
  .map((pkg) => ({ ecosystem: "Rust", name: pkg.name, version: pkg.version, license: pkg.license || "UNKNOWN", source: pkg.repository || pkg.homepage || pkg.source || "" }));

const lock = JSON.parse(readFileSync("package-lock.json", "utf8"));
const npm = Object.entries(lock.packages || {})
  .filter(([path, pkg]) => path.startsWith("node_modules/") && pkg.version)
  .map(([path, pkg]) => {
    const manifest = join(path, "package.json");
    const installed = existsSync(manifest) ? JSON.parse(readFileSync(manifest, "utf8")) : {};
    return { ecosystem: "npm", name: path.replace(/^node_modules\//, ""), version: pkg.version, license: pkg.license || installed.license || "UNKNOWN", source: pkg.resolved || installed.repository?.url || "" };
  });

const packages = [...rust, ...npm].sort((a, b) => `${a.ecosystem}/${a.name}`.localeCompare(`${b.ecosystem}/${b.name}`));
const lines = [
  "# AudioMaster third-party notices",
  "",
  "Generated from the locked Rust and npm dependency graphs. The corresponding release SBOM is authoritative; consult each linked package for its complete license text.",
  "",
  "| Ecosystem | Package | Version | License | Source |",
  "|---|---|---:|---|---|",
  ...packages.map((pkg) => `| ${pkg.ecosystem} | ${pkg.name} | ${pkg.version} | ${pkg.license} | ${pkg.source} |`),
  "",
];
writeFileSync(destination, lines.join("\n"));
console.log(`Wrote ${packages.length} dependency notices to ${destination}`);
