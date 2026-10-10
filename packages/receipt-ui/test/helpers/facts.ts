import type { Facts } from "@fettle/report-schema";

type Storage = NonNullable<NonNullable<Facts["storage"]>["data"]>;
type Battery = NonNullable<NonNullable<Facts["battery"]>["data"]>;
type Disk = Storage["disks"] extends (infer D)[] | undefined ? D : never;

export const ok = <T>(data: T) => ({ check: { status: "ok" as const }, data });
export const status = (s: "needs_admin" | "not_present" | "failed") => ({ check: { status: s } });

export const nvme = (percentage_used: number, temperature_c = 40): Disk => ({
  model: "Samsung SSD 990",
  bus: "nvme",
  media: "ssd",
  nvme: { percentage_used, temperature_c },
});
export const sata = (smart?: Disk["smart"]): Disk => ({ model: "Crucial MX500", bus: "sata", media: "ssd", smart });
export const storage = (disks: Disk[], volumes: Storage["volumes"] = []) => ok<Storage>({ disks, volumes });
export const battery = (design: number, full: number) =>
  ok<Battery>({ design_capacity_mwh: design, full_charge_capacity_mwh: full });

type Windows = NonNullable<NonNullable<Facts["windows"]>["data"]>;
type Memory = NonNullable<NonNullable<Facts["memory"]>["data"]>;
type Startup = NonNullable<NonNullable<Facts["startup"]>["data"]>;
type Graphics = NonNullable<NonNullable<Facts["graphics"]>["data"]>;

export const windows = (w: Windows) => ok<Windows>({ uptime_seconds: 3600, ...w });
export const memory = (m: Memory) => ok<Memory>(m);
export const startup = (enabled: number, disabled = 0) =>
  ok<Startup>({
    items: [
      ...Array.from({ length: enabled }, (_, i) => ({ name: `on${i}`, location: "run_key" as const, enabled: true })),
      ...Array.from({ length: disabled }, (_, i) => ({ name: `off${i}`, location: "run_key" as const, enabled: false })),
    ],
  });
export const graphics = (g: Graphics) => ok<Graphics>(g);
