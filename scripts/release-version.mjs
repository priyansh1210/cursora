// Refuses a release tag that doesn't match the app's version (UPD-3.4): v<version>, or v<version>-<suffix> for a
// test build, which is published as a pre-release. Usage: node scripts/release-version.mjs <tag>
import fs from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

export function tagMatches(tag, version) {
  return tag === `v${version}` || (tag.startsWith(`v${version}-`) && tag.length > `v${version}-`.length);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const { version } = JSON.parse(fs.readFileSync(fileURLToPath(new URL("../src-tauri/tauri.conf.json", import.meta.url)), "utf8"));
  const tag = process.argv[2] ?? "";
  if (!tagMatches(tag, version)) {
    console.error(`The tag "${tag}" doesn't match the app's version ${version}: use v${version}, or v${version}-rc.N for a test build.`);
    process.exit(1);
  }
  console.log(`${tag} matches the app's version ${version}`);
}
