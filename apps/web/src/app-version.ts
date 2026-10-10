// The Windows app's version, read from apps/windows/Cargo.toml: the only place it's written.
// Imported as text, so the build bundles it (a file path would break once the code moves to dist).
import cargo from "../../windows/Cargo.toml?raw";

const version = /^version\s*=\s*"([^"]+)"/m.exec(cargo)?.[1];
if (!version) throw new Error("apps/windows/Cargo.toml has no version");

export const appVersion: string = version;
