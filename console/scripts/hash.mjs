// TECHNICAL 18.2: after the build, ../ui/SHA256SUMS holds a sha256 line for each file the build wrote.
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, writeFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ui = fileURLToPath(new URL("../../ui/", import.meta.url));
const walk = (d) => readdirSync(d).flatMap((n) => (statSync(join(d, n)).isDirectory() ? walk(join(d, n)) : [join(d, n)]));
const files = walk(ui).map((f) => relative(ui, f).split("\\").join("/")).filter((f) => f !== "SHA256SUMS").sort();
const lines = files.map((f) => `${createHash("sha256").update(readFileSync(join(ui, f))).digest("hex")}  ${f}`);
writeFileSync(join(ui, "SHA256SUMS"), lines.join("\n") + "\n");
console.log(`SHA256SUMS: ${files.length} files`);
