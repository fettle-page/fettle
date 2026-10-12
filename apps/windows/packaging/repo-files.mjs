// Which tracked paths don't belong in the public repo: any hidden file or folder, and any Markdown
// file, unless it's one of the few listed here. Checked by test/repo-files.test.mjs on every run.
const ALLOWED_HIDDEN = [
  /^\.gitattributes$/,
  /^(.+\/)?\.gitignore$/,
  /^\.github\/workflows\/[\w.-]+\.ya?ml$/,
  /^\.github\/actions\/windows-deps\/action\.yml$/,
  /^\.github\/ISSUE_TEMPLATE\/(bug_report\.md|feature_request\.md|config\.yml)$/,
  /^\.github\/pull_request_template\.md$/,
  /^apps\/web\/public\/\.well-known\/[\w.-]+$/,
  /^spike\/\.cargo\/config\.toml$/,
];
const ALLOWED_MARKDOWN = /^((.+\/)?README|SECURITY|CONTRIBUTING|\.github\/ISSUE_TEMPLATE\/(bug_report|feature_request)|\.github\/pull_request_template)\.md$/;

/** Account SIDs in `text` that could be real: test data may only use ones whose numbers are each a
 * single repeated digit (S-1-5-21-111111111-2222222222-333333333-1001, S-1-5-21-1-2-3-1001, and
 * Entra ID's S-1-12-1-1111111111-2222222222-3333333333-4444444444), in either letter case. */
export function realLookingSids(text) {
  const local = [...text.matchAll(/S-1-5-21-(\d+)-(\d+)-(\d+)/gi)].map((m) => [m[0], m.slice(1, 4)]);
  const entra = [...text.matchAll(/S-1-12-1-(\d+)-(\d+)-(\d+)-(\d+)/gi)].map((m) => [m[0], m.slice(1, 5)]);
  return [...local, ...entra].filter(([, ns]) => ns.some((n) => !/^(\d)\1*$/.test(n))).map(([sid]) => sid);
}

/** CPU ProcessorId values in `text` that could be real: only placeholders such as "<private>" and
 * one repeated character are allowed. */
export function realLookingProcessorIds(text) {
  return [...text.matchAll(/"ProcessorId"\s*:\s*"([^"]*)"/g)].map((m) => m[1]).filter((v) => !/^<[\w-]+>$/.test(v) && !/^(.)\1*$/.test(v));
}

/** Browser startup names in `text` (MicrosoftEdgeAutoLaunch_<32 hex>, GoogleChromeAutoLaunch_…) that
 * could be real: the code is derived from the profile path, so test data may only use one repeated
 * character or 0123…EF twice. */
export function realLookingAutoLaunchCodes(text) {
  return [...text.matchAll(/AutoLaunch_([0-9A-Fa-f]{32})/g)].map((m) => m[1]).filter((v) => !/^(.)\1*$/.test(v) && v.toUpperCase() !== "0123456789ABCDEF".repeat(2));
}

/** The paths in `paths` that aren't allowed in the repo, in their original order. */
export function unexpectedPaths(paths) {
  return paths.filter((p) => {
    const hidden = p.split("/").some((part) => part.startsWith("."));
    if (hidden && !ALLOWED_HIDDEN.some((re) => re.test(p))) return true;
    return /\.md$/i.test(p) && !ALLOWED_MARKDOWN.test(p);
  });
}

/** Domains test data may use in addresses: the reserved example ones, Microsoft's made-up Contoso,
 * the project's own, and GitHub's no-reply one. */
const FAKE_MAIL_DOMAINS = /^(.+\.)?(example(\.(com|org|net|co\.uk))?|contoso(-group)?\.com|fettle\.page|users\.noreply\.github\.com)$/i;

/** Endings that make what follows the @ a file, not a mail domain (Inno Setup imports a function
 * as GetFileAttributesW@kernel32.dll). */
const FILE_ENDINGS = /\.(dll|exe|sys|ocx|drv|png|jpe?g|gif|svg|webp|ico|css|js|mjs|ts|json|wasm|txt)$/i;

/** Email addresses in `text` whose domain isn't an example, test or project one. A domain starts
 * with a letter, so "logo@2x.png" and "pkg@1.0.0-beta.rc" aren't addresses, nor are file names. */
export function realLookingEmails(text) {
  return [...text.matchAll(/[\w.%+'-]+@([A-Za-z][A-Za-z0-9-]*(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,})\b/g)].filter((m) => !FAKE_MAIL_DOMAINS.test(m[1]) && !FILE_ENDINGS.test(m[1])).map((m) => m[0]);
}

/** Serial-number values in `text` that could be real: test data may only use a placeholder, a
 * value of up to four characters, one repeated character, or one that starts with a made-up
 * marker (FAKE, TEST, XYZ, ABCD, 1234). Checked in JSON ("SerialNumber": "…") and in Rust or
 * JavaScript fields (serial: "…"). */
export function realLookingSerials(text) {
  const json = /"((?:\w*serial\w*)|FruId|UUID|IdentifyingNumber)"\s*:\s*"([^"]*)"/gi;
  const code = /\b(serial(?:_?number)?)\s*:\s*(?:Some\()?"([^"]*)"/gi;
  const fake = (v) => /^<[\w-]+>$/.test(v) || v.length <= 4 || /^(.)\1*$/.test(v) || /^(FAKE|F1AKE|TEST|T1EST|TES\d|XYZ|ABCD|1234)/i.test(v);
  return [...text.matchAll(json), ...text.matchAll(code)].map((m) => m[2].trim()).filter((v) => !fake(v));
}
