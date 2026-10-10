// The brain in the share Worker: Wrangler turns the .wasm import into a compiled module.
import wasm from "../dist/fettle.wasm";
import { createBrain } from "./core.ts";

export const brain = createBrain(wasm);
export { BrainError, type Brain, type Zone } from "./core.ts";
