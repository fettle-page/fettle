import { env } from "cloudflare:workers";
import { runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { describe, expect, it, vi } from "vitest";
import { GUESSES_PER_DAY, GUESSES_PER_VISITOR_PER_DAY, type UnlockGuard } from "../src/guard.ts";
import { DAY } from "./helpers.ts";

let n = 0;
const fresh = () => env.UNLOCK_GUARD.getByName(`guard-test-${Date.now()}-${n++}`);
const MINUTE = 60_000;
// An hour into today (UTC), so a day's worth of one-a-minute guesses stays inside the day.
const today = () => Math.floor(Date.now() / DAY) * DAY + 3_600_000;

/** `count` wrong guesses, one a minute from `start`, each from `visitor(i)`: the waits begin() gave. */
const guesses = (stub: DurableObjectStub<UnlockGuard>, count: number, visitor: (i: number) => string, start: number) =>
  runInDurableObject(stub, async (guard: UnlockGuard) => {
    const waits: number[] = [];
    for (let i = 0; i < count; i++) waits.push(await guard.begin(visitor(i), start + i * MINUTE));
    return waits;
  });

describe("UnlockGuard", () => {
  it("caps one visitor's wrong guesses a day without locking out anyone else", async () => {
    const stub = fresh();
    const start = today();
    expect((await guesses(stub, GUESSES_PER_VISITOR_PER_DAY, () => "198.51.100.7", start)).every((w) => w === 0)).toBe(true);
    const at = start + GUESSES_PER_VISITOR_PER_DAY * MINUTE;
    const [wait] = await guesses(stub, 1, () => "198.51.100.7", at);
    expect(wait).toBe(Math.ceil((Math.floor(at / DAY + 1) * DAY - at) / 1000));
    expect(await guesses(stub, 1, () => "198.51.100.8", at)).toEqual([0]);
  });

  it("caps a report's wrong guesses a day across all visitors", async () => {
    const stub = fresh();
    const start = today();
    expect((await guesses(stub, GUESSES_PER_DAY, (i) => `visitor-${i}`, start)).every((w) => w === 0)).toBe(true);
    const [wait] = await guesses(stub, 1, () => "someone-new", start + GUESSES_PER_DAY * MINUTE);
    expect(wait).toBeGreaterThan(60);
  });

  it("starts every count over the next day", async () => {
    const stub = fresh();
    const start = today();
    await guesses(stub, GUESSES_PER_VISITOR_PER_DAY, () => "198.51.100.7", start);
    expect(await guesses(stub, 1, () => "198.51.100.7", start + DAY)).toEqual([0]);
  });

  it("keeps no address, and clears a past day's counts", async () => {
    const stub = fresh();
    // Looked at in the same call: the past day's clean-up alarm is due at once.
    const keys = await runInDurableObject(stub, async (guard: UnlockGuard, state) => {
      for (let i = 0; i < 3; i++) await guard.begin("198.51.100.7", today() - 2 * DAY + i * MINUTE);
      return [...(await state.storage.list()).keys()];
    });
    expect(keys.length).toBeGreaterThan(0);
    expect(keys.join(" ")).not.toContain("198.51.100");
    await runDurableObjectAlarm(stub);
    // The day ended two days ago, so the clean-up alarm was due the moment it was set: workerd may run
    // it on its own, and then runDurableObjectAlarm finds none while that run is still on its way.
    // Wait for the clean-up, whichever of the two runs it.
    await vi.waitFor(async () => expect(await runInDurableObject(stub, async (_, state) => (await state.storage.list()).size)).toBe(0));
  });

  it("gives a right password's tries back", async () => {
    const stub = fresh();
    const start = today();
    await guesses(stub, GUESSES_PER_VISITOR_PER_DAY - 1, () => "198.51.100.7", start);
    const at = start + GUESSES_PER_VISITOR_PER_DAY * MINUTE;
    await runInDurableObject(stub, async (guard: UnlockGuard) => {
      expect(await guard.begin("198.51.100.7", at)).toBe(0);
      await guard.succeeded("198.51.100.7", at);
    });
    expect(await guesses(stub, 1, () => "198.51.100.7", at + MINUTE)).toEqual([0]);
  });
});
