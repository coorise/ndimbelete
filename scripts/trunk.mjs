/**
 * Trunk wrapper — Cursor/CI often set NO_COLOR=1, but Trunk 0.21's clap
 * only accepts true/false for --no-color, which breaks `trunk serve`.
 */
import fs from "node:fs";
import { spawn } from "node:child_process";

// Ensure required build and target directories exist so Trunk canonicalize never fails
for (const dir of ["target", "dist", "src-tauri"]) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

const args = process.argv.slice(2);
const env = { ...process.env, TRUNK_COLOR: "never" };
delete env.NO_COLOR;

const child = spawn("trunk", args, {
  env,
  stdio: "inherit",
  shell: true,
});

const cleanup = () => {
  if (child && child.pid) {
    try {
      if (process.platform === "win32") {
        spawn("taskkill", ["/pid", child.pid.toString(), "/f", "/t"], { stdio: "ignore" });
      } else {
        child.kill("SIGTERM");
      }
    } catch (_) {}
  }
};

process.on("SIGINT", () => {
  cleanup();
  process.exit(0);
});

process.on("SIGTERM", () => {
  cleanup();
  process.exit(0);
});

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 0);
});
