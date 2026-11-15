# Skill: Changelog and version bumps (godsend-psp)

> Global workflow: `~/.claude/skills/changelog/SKILL.md`

## Repo paths

| Item | Path |
|------|------|
| Changelog | `CHANGELOG.md` (repo root) |
| Version source of truth | `Cargo.toml` `[workspace.package].version` |
| Mirrored versions | `app/src-tauri/tauri.conf.json`, `app/package.json`, `app/src/App.tsx` (`VERSION`), `app/src/pages/Settings.tsx` about line |
| Version bump helper | `scripts/bump-version.sh X.Y.Z` |
| Pre-commit check | `.githooks/pre-commit` runs `scripts/check-changelog.sh` |

## Rules

1. Every change to product code, config, or user-visible copy bumps the
   version and writes an entry in `CHANGELOG.md`, in the same commit.
2. **No `[Unreleased]` section.** Newest release header at the top:
   `## [x.y.z] - YYYY-MM-DD`. Add the bullet under `Added` / `Changed` /
   `Fixed` / `Removed` / `Security` beneath that header.
3. Patch bump for fixes and internal work, minor bump for new features,
   major only for breaking changes to data layout or CLI contracts.
4. User-facing language: what changed for the user or operator, not file
   lists. No em or en dashes.
5. Bump both the version and the changelog in one step:
   ```sh
   scripts/bump-version.sh 0.2.1 "Fixed: the thing"
   ```
   The helper updates every version location and opens a dated header.
6. Pre-commit **fails** when staged product code (`core/`, `cli/`,
   `app/src/`, `app/src-tauri/src/`, `app/src-tauri/tauri.conf.json`,
   `scripts/`, `.github/`) comes without a staged `CHANGELOG.md` diff, or
   when the mirrored version files disagree.

## Release

`scripts/publish-release.sh` reads the version from `tauri.conf.json` and
uploads that build to GoFile and file.kiwi. Bump first, publish second.
