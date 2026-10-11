// Which tracked paths don't belong in the public repo: any hidden file or folder, and any Markdown
// file, unless it's one of the few listed here. Checked by test/repo-files.test.mjs on every run.
const ALLOWED_HIDDEN = [
  /^\.gitattributes$/,
  /^(.+\/)?\.gitignore$/,
  /^\.github\/workflows\/[\w.-]+\.ya?ml$/,
  /^apps\/web\/public\/\.well-known\/[\w.-]+$/,
  /^spike\/\.cargo\/config\.toml$/,
];
const ALLOWED_MARKDOWN = /^(.+\/)?README\.md$/;

/** The paths in `paths` that aren't allowed in the repo, in their original order. */
export function unexpectedPaths(paths) {
  return paths.filter((p) => {
    const hidden = p.split("/").some((part) => part.startsWith("."));
    if (hidden && !ALLOWED_HIDDEN.some((re) => re.test(p))) return true;
    return /\.md$/i.test(p) && !ALLOWED_MARKDOWN.test(p);
  });
}
