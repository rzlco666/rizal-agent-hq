import { readFile, writeFile, mkdir } from "node:fs/promises";
import openapiTS, { astToString } from "openapi-typescript";
import { dirname, resolve } from "node:path";

const args = process.argv.slice(2);
const input = args.find((arg) => !arg.startsWith("--"));
const outIndex = args.indexOf("--out");
if (!input || outIndex === -1 || !args[outIndex + 1]) {
  console.error("usage: node gen.mjs <openapi.json> --out <schema.d.ts>");
  process.exit(2);
}

const source = await readFile(resolve(input), "utf8");
const document = JSON.parse(source);
if (!document.paths || Object.keys(document.paths).length === 0) {
  console.error("openapi document has no paths");
  process.exit(1);
}
const types = astToString(await openapiTS(document));
const output = resolve(args[outIndex + 1]);
await mkdir(dirname(output), { recursive: true });
await writeFile(output, types, "utf8");
console.log(`wrote ${output}`);
