// The brain in Node: the website build and tests. Loaded once, from the file `npm run build` makes.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { createBrain } from "./core.ts";

// Found through the package's own name, not next to this file: the website build bundles this
// module elsewhere (dist/.prerender), where a relative path would miss.
const home = dirname(createRequire(import.meta.url).resolve("@fettle/brain/package.json"));

export const brain = createBrain(new WebAssembly.Module(readFileSync(join(home, "dist/fettle.wasm"))));
export { BrainError, type Brain, type Zone } from "./core.ts";
