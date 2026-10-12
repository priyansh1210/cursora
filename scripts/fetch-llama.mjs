// Downloads llama.cpp's CPU build for this OS into src-tauri/llama/ (gitignored), checks its pinned SHA-256,
// and keeps llama-server with its libraries (spec 04, LLM-3.1). The release job runs it before bundling;
// in dev, run it once to try the built-in engine: `npm run fetch-llama`.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const BUILD = "b11570";
const ASSETS = {
  win32: { file: `llama-${BUILD}-bin-win-cpu-x64.zip`, sha256: "2ac50506661724876ec12b50fa2f4e2d3ddac4830da8cb799d3703bba4c72da4", strip: 0 },
  linux: { file: `llama-${BUILD}-bin-ubuntu-x64.tar.gz`, sha256: "6fa129f3e6f5a9dab675d0d41436bc797c826a62438ac295d129bbba7b3c637b", strip: 1 },
};

/** llama-server, its libraries and the licenses; not the other tools or their -impl libraries. */
export function keep(name) {
  if (/^llama-server(\.exe)?$/.test(name) || /^LICENSE/.test(name)) return true;
  const library = /\.dll$|\.so(\.\d+)*$/.test(name);
  return library && (!/-impl\./.test(name) || /llama-server-impl\./.test(name));
}

async function main() {
  const asset = ASSETS[process.platform];
  if (!asset) throw new Error(`No llama.cpp build is pinned for ${process.platform}.`);
  const out = fileURLToPath(new URL("../src-tauri/llama/", import.meta.url));
  const url = `https://github.com/ggml-org/llama.cpp/releases/download/${BUILD}/${asset.file}`;
  console.log(`Downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Download failed: ${res.status}`);
  const data = Buffer.from(await res.arrayBuffer());
  const sha = createHash("sha256").update(data).digest("hex");
  if (sha !== asset.sha256) throw new Error(`SHA-256 mismatch: got ${sha}, expected ${asset.sha256}`);

  const archive = join(out, "..", asset.file);
  writeFileSync(archive, data);
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  // tar reads both the .zip and the .tar.gz. On Windows, its own bsdtar: Git's GNU tar takes "C:" for a remote host.
  const tar = process.platform === "win32" ? join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe") : "tar";
  execFileSync(tar, ["-xf", archive, "-C", out, `--strip-components=${asset.strip}`], { stdio: "inherit" });
  rmSync(archive);
  // shortcut: Linux symlinks (libllama.so → .so.0 → .so.0.6.0) are bundled as copies, about 15 MB extra; resolve them if size matters.
  for (const name of readdirSync(out)) if (!keep(name)) rmSync(join(out, name), { recursive: true });
  // llama.cpp is MIT-licensed, and the Windows archive leaves its license out.
  if (!readdirSync(out).includes("LICENSE")) {
    const license = await fetch(`https://raw.githubusercontent.com/ggml-org/llama.cpp/${BUILD}/LICENSE`);
    if (!license.ok) throw new Error(`License download failed: ${license.status}`);
    writeFileSync(join(out, "LICENSE"), await license.text());
  }
  console.log(`llama.cpp ${BUILD} is in ${out}`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) await main();
