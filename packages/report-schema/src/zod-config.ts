// Imported first by index.ts: zod reads this when the schemas are built, so it must run before
// generated.ts loads. Fettle's pages forbid eval (strict CSP); without it, zod probes `new Function`
// and the browser logs a violation. Parsing works the same, just without zod's JIT.
import { z } from "zod";

z.config({ jitless: true });
