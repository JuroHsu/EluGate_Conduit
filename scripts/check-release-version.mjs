import { appendFileSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

// Keep this check dependency-free so CI can reject a wrong tag before installing.
export function checkReleaseVersion(root, tag) {
  const json = (name) => JSON.parse(readFileSync(resolve(root, name), "utf8"));
  const pkg = json("package.json");
  const lock = json("package-lock.json");
  const tauri = json("src-tauri/tauri.conf.json");
  const cargoVersion = (name) => {
    const text = readFileSync(resolve(root, name), "utf8");
    const sections = text.split(/(?=^\[(?:\[)?package\](?:\])?\s*$)/m);
    const section = sections.find((s) => /^name\s*=\s*"elugate-conduit"\s*$/m.test(s));
    const version = section?.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
    if (!version) throw new Error(`Cannot find elugate-conduit's version in ${name}`);
    return version;
  };
  const versions = {
    "package.json": pkg.version,
    "package-lock.json": lock.version,
    "package-lock.json root package": lock.packages?.[""]?.version,
    "src-tauri/tauri.conf.json": tauri.version,
    "src-tauri/Cargo.toml": cargoVersion("src-tauri/Cargo.toml"),
    "src-tauri/Cargo.lock": cargoVersion("src-tauri/Cargo.lock"),
  };
  const version = pkg.version;
  // Numeric prerelease sequence also works with the Windows MSI version mapping.
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(alpha|beta|rc)\.(0|[1-9]\d*))?$/.test(version)) {
    throw new Error("Use a version such as 0.1.0 or 0.2.0-beta.1.");
  }
  for (const [file, value] of Object.entries(versions)) {
    if (value !== version) throw new Error(`${file} has version ${value}; expected ${version}`);
  }
  if (tag !== undefined && tag !== `v${version}`) {
    throw new Error(`Tag ${tag} does not match the app version. Expected v${version}.`);
  }
  return { version, prerelease: version.includes("-") };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = fileURLToPath(new URL("../", import.meta.url));
  const tag = process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : undefined;
  const result = checkReleaseVersion(root, tag);
  console.log(`Validated version ${result.version}${tag ? ` for ${tag}` : " (build only)"}.`);
  if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT, `version=${result.version}\nprerelease=${result.prerelease}\n`);
  }
}
