/**
 * Cut a release, keeping the version in sync across package.json,
 * tauri.conf.json, Cargo.toml and Cargo.lock, then commit, tag, and push.
 * Pushing the tag triggers the Release workflow, which builds, signs, and
 * publishes the GitHub release.
 *
 *   bun run release 0.1.1      # explicit version (recommended — deterministic)
 *   bun run release            # let changelogen pick the next version from commits
 *   bun run release --patch    # force a patch/minor/major bump via changelogen
 *
 * Pass an exact version to avoid any guesswork about the released number.
 */
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const run = (cmd) => execSync(cmd, { stdio: "inherit" });
const args = process.argv.slice(2);
const explicit = args.find((a) => /^\d+\.\d+\.\d+([-+.].+)?$/.test(a));

const setVersion = (file, re, replacement) =>
  writeFileSync(file, readFileSync(file, "utf8").replace(re, replacement));

if (explicit) {
  // Deterministic path: WE choose the version, changelogen only writes notes.
  setVersion("package.json", /("version":\s*)"[^"]+"/, `$1"${explicit}"`);
  run("bun x changelogen --output CHANGELOG.md"); // headed by the version we just set
} else {
  // changelogen determines the next version from Conventional Commits.
  run(`bun x changelogen --bump --output CHANGELOG.md ${args.join(" ")}`.trim());
}

const version = JSON.parse(readFileSync("package.json", "utf8")).version;

// Mirror that version into the Rust/Tauri side.
const conf = "src-tauri/tauri.conf.json";
const cargo = "src-tauri/Cargo.toml";
const lock = "src-tauri/Cargo.lock";
setVersion(conf, /("version":\s*)"[^"]+"/, `$1"${version}"`);
setVersion(cargo, /^version = "[^"]+"$/m, `version = "${version}"`);
setVersion(lock, /(name = "starnet-gui"\nversion = )"[^"]+"/, `$1"${version}"`);

// Point the README's four download buttons at this version's assets. The tag
// and the filename both carry the version, so they have to move together. A
// silent miss here — a renamed bundle, an edited README — would ship four dead
// buttons on the repo homepage, so assert the rewrite actually landed.
const readme = "README.md";
const stamped = readFileSync(readme, "utf8")
  .replace(/(\/releases\/download\/)v[^/]+\//g, `$1v${version}/`)
  .replace(/(starnetGUI_)[^_]+_/g, `$1${version}_`);
const stampedLinks = stamped.match(
  new RegExp(`/releases/download/v${version}/starnetGUI_${version}_`, "g"),
);
if (stampedLinks?.length !== 4) {
  throw new Error(
    `README.md: expected 4 download links for v${version}, rewrote ${stampedLinks?.length ?? 0}. ` +
      `Check the bundle filenames in the release workflow against the links in README.md.`,
  );
}
writeFileSync(readme, stamped);

// Commit, tag, and push. The tag push kicks off the Release workflow.
run(`git add package.json CHANGELOG.md ${readme} ${conf} ${cargo} ${lock}`);
run(`git commit -m "chore(release): v${version}"`);
run(`git tag v${version}`);
run("git push");
run(`git push origin v${version}`); // push the tag explicitly (lightweight tags aren't sent by --follow-tags)

console.log(`\nReleased v${version}. GitHub Actions is now building and publishing it.`);
