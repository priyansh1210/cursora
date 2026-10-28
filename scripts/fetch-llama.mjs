// Downloads llama.cpp's Vulkan build for this OS into src-tauri/llama/ (gitignored), checks its pinned SHA-256,
// and keeps llama-server with its libraries (spec 04, LLM-3.1). The release job runs it before bundling;
// in dev, run it once to try the built-in engine: `npm run fetch-llama`.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const BUILD = "b11570";
// The Vulkan builds (spec 23): the CPU backends too, and a Vulkan one llama.cpp loads only where Vulkan is installed.
const ASSETS = {
  win32: { file: `llama-${BUILD}-bin-win-vulkan-x64.zip`, sha256: "21b7c457e87873e9fc29434957db83afe8cf06f5bd74fba89b9d64307e6fdabf", strip: 0 },
  linux: { file: `llama-${BUILD}-bin-ubuntu-vulkan-x64.tar.gz`, sha256: "2c4587bb3617c03fc5dd5ca263ea3b4a3abcbe548c12b827d824d295508989ae", strip: 1 },
};

// What the llama.cpp files import from the Visual C++ runtime (dumpbin /dependents); the api-ms-win-crt-* part is in Windows.
const VC_RUNTIME = ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"];

/** Visual Studio's folder of redistributable runtime files, the ones Microsoft allows apps to ship. */
function vcRedist() {
  const vswhere = join(process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)", "Microsoft Visual Studio", "Installer", "vswhere.exe");
  const vs = execFileSync(vswhere, ["-latest", "-products", "*", "-property", "installationPath"], { encoding: "utf8" }).trim();
  const version = readFileSync(join(vs, "VC", "Auxiliary", "Build", "Microsoft.VCRedistVersion.default.txt"), "utf8").trim();
  const x64 = join(vs, "VC", "Redist", "MSVC", version, "x64");
  const crt = readdirSync(x64).find((name) => /^Microsoft\.VC\d+\.CRT$/.test(name));
  if (!crt) throw new Error(`No Visual C++ runtime in ${x64}`);
  return join(x64, crt);
}

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
  // A fresh Windows lacks the Visual C++ runtime llama.cpp needs (issue 50): ship it next to llama-server ("app-local").
  if (process.platform === "win32") for (const dll of VC_RUNTIME) copyFileSync(join(vcRedist(), dll), join(out, dll));
  // llama.cpp is MIT-licensed, and the Windows archive leaves its license out.
  if (!readdirSync(out).includes("LICENSE")) {
    const license = await fetch(`https://raw.githubusercontent.com/ggml-org/llama.cpp/${BUILD}/LICENSE`);
    if (!license.ok) throw new Error(`License download failed: ${license.status}`);
    writeFileSync(join(out, "LICENSE"), await license.text());
  }
  console.log(`llama.cpp ${BUILD} is in ${out}`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) await main();
