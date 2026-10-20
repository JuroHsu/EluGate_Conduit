import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { checkReleaseVersion } from "./check-release-version.mjs";

function fixture(t, version = "0.1.0") {
  const root = mkdtempSync(join(tmpdir(), "conduit-version-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "src-tauri"));
  const write = (path, value) => writeFileSync(join(root, path), value);
  write("package.json", JSON.stringify({ version }));
  write("package-lock.json", JSON.stringify({ version, packages: { "": { version } } }));
  write("src-tauri/tauri.conf.json", JSON.stringify({ version }));
  write("src-tauri/Cargo.toml", `[package]\nname = "elugate-conduit"\nversion = "${version}"\n\n[lib]\nname = "conduit_lib"\n`);
  write("src-tauri/Cargo.lock", `version = 4\n\n[[package]]\nname = "dependency"\nversion = "9.9.9"\n\n[[package]]\nname = "elugate-conduit"\nversion = "${version}"\n`);
  return { root, write };
}

test("accepts matching release tag and ignores dependency versions", (t) => {
  const { root } = fixture(t);
  assert.deepEqual(checkReleaseVersion(root, "v0.1.0"), { version: "0.1.0", prerelease: false });
});

test("permits a branch build without a release tag", (t) => {
  assert.equal(checkReleaseVersion(fixture(t).root).version, "0.1.0");
});

test("rejects a tag for a different version", (t) => {
  assert.throws(() => checkReleaseVersion(fixture(t).root, "v0.2.0"), /does not match/);
});

test("rejects a stale root package entry in the npm lockfile", (t) => {
  const { root, write } = fixture(t);
  write("package-lock.json", JSON.stringify({ version: "0.1.0", packages: { "": { version: "0.0.9" } } }));
  assert.throws(() => checkReleaseVersion(root), /root package has version 0.0.9/);
});

test("rejects stale Tauri and Cargo versions", (t) => {
  const { root, write } = fixture(t);
  write("src-tauri/tauri.conf.json", '{"version":"0.0.9"}');
  assert.throws(() => checkReleaseVersion(root), /tauri.conf.json has version 0.0.9/);
  write("src-tauri/tauri.conf.json", '{"version":"0.1.0"}');
  write("src-tauri/Cargo.lock", '[[package]]\nname = "elugate-conduit"\nversion = "0.0.9"\n');
  assert.throws(() => checkReleaseVersion(root), /Cargo.lock has version 0.0.9/);
});

test("identifies prereleases and rejects malformed versions", (t) => {
  const { root } = fixture(t, "0.2.0-beta.1");
  assert.deepEqual(checkReleaseVersion(root, "v0.2.0-beta.1"), { version: "0.2.0-beta.1", prerelease: true });
  assert.throws(() => checkReleaseVersion(fixture(t, "v0.2").root), /Use a version/);
});
