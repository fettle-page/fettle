// Keeps the build machine's folders out of fettle.exe. Rust writes source paths into a program (for
// panic messages), and on a PC those sit under the user's own folder, user name included. The
// published builds come from GitHub's machines, but a build made on a PC and passed on mustn't carry
// that PC's user name either.

/**
 * rustc flags that write the build machine's folders under neutral names. When several match, rustc
 * uses the last, so the user folder comes first and the folders inside it after.
 */
export function remapFlags({ home, cargoHome, rustupHome, root }) {
  return [
    [home, "/home"],
    [cargoHome, "/cargo"],
    [rustupHome, "/rustup"],
    [root, "/fettle"],
  ]
    .filter(([from]) => from)
    .map(([from, to]) => `--remap-path-prefix=${from}=${to}`);
}

/** Flags for CARGO_ENCODED_RUSTFLAGS: separated by 0x1f, so a folder name with a space stays whole. */
export function encodedRustFlags(flags) {
  return flags.join("\x1f");
}

/** How many times `path` appears in a program, in any case, as plain or wide (UTF-16) text. */
export function timesIn(bytes, path) {
  const count = (text, needle) => text.split(needle).length - 1;
  const lower = path.toLowerCase();
  return count(bytes.toString("latin1").toLowerCase(), lower) + count(bytes.toString("utf16le").toLowerCase(), lower) + count(bytes.subarray(1).toString("utf16le").toLowerCase(), lower);
}
