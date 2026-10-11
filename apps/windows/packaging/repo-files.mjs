// Which tracked paths don't belong in the public repo: any hidden file or folder, and any Markdown
// file, unless it's one of the few listed here. Checked by test/repo-files.test.mjs on every run.
const ALLOWED_HIDDEN = [
  /^\.gitattributes$/,
  /^(.+\/)?\.gitignore$/,
  /^\.github\/workflows\/[\w.-]+\.ya?ml$/,
  /^apps\/web\/public\/\.well-known\/[\w.-]+$/,
  /^spike\/\.cargo\/config\.toml$/,
];
const ALLOWED_MARKDOWN = /^((.+\/)?README|SECURITY)\.md$/;

/** Account SIDs in `text` that could be real: test data may only use ones whose numbers are each a
 * single repeated digit (S-1-5-21-111111111-2222222222-333333333-1001, S-1-5-21-1-2-3-1001, and
 * Entra ID's S-1-12-1-1111111111-2222222222-3333333333-4444444444). */
export function realLookingSids(text) {
  const local = [...text.matchAll(/S-1-5-21-(\d+)-(\d+)-(\d+)/g)].map((m) => [m[0], m.slice(1, 4)]);
  const entra = [...text.matchAll(/S-1-12-1-(\d+)-(\d+)-(\d+)-(\d+)/g)].map((m) => [m[0], m.slice(1, 5)]);
  return [...local, ...entra].filter(([, ns]) => ns.some((n) => !/^(\d)\1*$/.test(n))).map(([sid]) => sid);
}

/** CPU ProcessorId values in `text` that could be real: only placeholders such as "<private>" and
 * one repeated character are allowed. */
export function realLookingProcessorIds(text) {
  return [...text.matchAll(/"ProcessorId"\s*:\s*"([^"]*)"/g)].map((m) => m[1]).filter((v) => !/^<[\w-]+>$/.test(v) && !/^(.)\1*$/.test(v));
}

/** Edge's MicrosoftEdgeAutoLaunch_<32 hex> startup names in `text` that could be real: the code is
 * derived from the profile path, so test data may only use one repeated character or 0123…EF twice. */
export function realLookingEdgeCodes(text) {
  return [...text.matchAll(/MicrosoftEdgeAutoLaunch_([0-9A-Fa-f]{32})/g)].map((m) => m[1]).filter((v) => !/^(.)\1*$/.test(v) && v.toUpperCase() !== "0123456789ABCDEF".repeat(2));
}

/** The paths in `paths` that aren't allowed in the repo, in their original order. */
export function unexpectedPaths(paths) {
  return paths.filter((p) => {
    const hidden = p.split("/").some((part) => part.startsWith("."));
    if (hidden && !ALLOWED_HIDDEN.some((re) => re.test(p))) return true;
    return /\.md$/i.test(p) && !ALLOWED_MARKDOWN.test(p);
  });
}
