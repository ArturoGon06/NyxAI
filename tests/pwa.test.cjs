const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const publicDirectory = path.join(__dirname, "..", "public");

test("manifest declares a standalone NyxAI application shell", () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(publicDirectory, "manifest.webmanifest"), "utf8"));

  assert.equal(manifest.name, "NyxAI");
  assert.equal(manifest.short_name, "NyxAI");
  assert.equal(manifest.start_url, "/");
  assert.equal(manifest.display, "standalone");
  assert.equal(manifest.theme_color, "#08080B");
  assert.ok(manifest.icons.some((icon) => icon.src === "/icons/nyxai.svg"));
});

test("service worker intentionally excludes private API and avatar traffic", () => {
  const worker = fs.readFileSync(path.join(publicDirectory, "sw.js"), "utf8");

  assert.match(worker, /url\.pathname\.startsWith\("\/api\/"\)/);
  assert.match(worker, /url\.pathname\.startsWith\("\/avatars\/"\)/);
  assert.match(worker, /CACHE_NAME = "nyxai-shell-v1\.4"/);
  assert.match(worker, /"\/app\.js\?v=1\.4\.0"/);
});
