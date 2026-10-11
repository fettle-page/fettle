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

/** Account SIDs in `text` that could be real: test data may only use ones whose three numbers are
 * each a single repeated digit (S-1-5-21-111111111-2222222222-333333333-1001, S-1-5-21-1-2-3-1001). */
export function realLookingSids(text) {
  return [...text.matchAll(/S-1-5-21-(\d+)-(\d+)-(\d+)/g)].filter((m) => m.slice(1, 4).some((n) => !/^(\d)\1*$/.test(n))).map((m) => m[0]);
}

/** The paths in `paths` that aren't allowed in the repo, in their original order. */
export function unexpectedPaths(paths) {
  return paths.filter((p) => {
    const hidden = p.split("/").some((part) => part.startsWith("."));
    if (hidden && !ALLOWED_HIDDEN.some((re) => re.test(p))) return true;
    return /\.md$/i.test(p) && !ALLOWED_MARKDOWN.test(p);
  });
}
