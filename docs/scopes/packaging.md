# Packaging Scope

## Distribution

- GitHub releases — source tarballs via git tags (`vX.Y.Z`).
- AUR — `hyprscreen` (stable, versioned). Installed via `paru -S hyprscreen`.

## Repos

- `pkg/PKGBUILD` lives in the main repo (reference copy).
- The actual AUR submission is a separate git clone at `~/git/aur-hyprscreen` pushed to `ssh://aur@aur.archlinux.org/hyprscreen.git`.

## Release (automated)

Releases are automated by `.github/workflows/release.yml`, triggered on any
`vX.Y.Z` tag push. To cut a release:

```bash
# 1. Bump the version in the code, commit, and push main
sed -i 's/^version = ".*"/version = "X.Y.Z"/' Cargo.toml
cargo build --release            # refreshes Cargo.lock
git commit -am "chore: vX.Y.Z"
git push origin main

# 2. Tag and push — this fires the workflow
git tag vX.Y.Z
git push origin vX.Y.Z
```

The workflow then:

1. Builds the release binary on a clean runner (build gate).
2. Creates the GitHub Release with the prebuilt `hyprscreen-vX.Y.Z-x86_64` binary attached.
3. Computes the sha256 of the tag's source tarball.
4. Bumps `pkg/PKGBUILD` (pkgver + sha256), regenerates `.SRCINFO`, and pushes to the AUR.

You no longer edit the PKGBUILD hash or touch `~/git/aur-hyprscreen` by hand.

### One-time setup

The AUR push authenticates with an SSH key stored as the `AUR_SSH_PRIVATE_KEY`
repository secret (Settings → Secrets and variables → Actions). Use the private
key whose public half is registered on your AUR account. Without it, the
`publish-aur` job fails but the GitHub Release still succeeds.

## Release Runbook (manual fallback)

If the workflow is unavailable, release `vX.Y.Z` in this exact order.

### 1. Bump version locally

```bash
# Cargo.toml
sed -i 's/^version = ".*"/version = "X.Y.Z"/' Cargo.toml
cargo build --release   # refreshes Cargo.lock with new version

# pkg/PKGBUILD
sed -i 's/^pkgver=.*/pkgver=X.Y.Z/' pkg/PKGBUILD
sed -i "s/^sha256sums=.*/sha256sums=('SKIP')/" pkg/PKGBUILD
```

`SKIP` is a placeholder — the real hash can't exist until the tag is pushed (the hash is of the tarball that GitHub generates from the tag).

### 2. Commit + push the code

```bash
git add -A
git commit -m "<type>: <message> (vX.Y.Z)"
git push origin main
```

### 3. Tag + push

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

This creates the GitHub source tarball at `https://github.com/franlol/hyprscreen/archive/refs/tags/vX.Y.Z.tar.gz`.

### 4. Compute the real hash

```bash
curl -sL https://github.com/franlol/hyprscreen/archive/refs/tags/vX.Y.Z.tar.gz | sha256sum
```

### 5. Update `pkg/PKGBUILD` with the hash + push

```bash
sed -i "s/^sha256sums=.*/sha256sums=('<HASH>')/" pkg/PKGBUILD
git add pkg/PKGBUILD
git commit -m "chore(pkg): vX.Y.Z sha256"
git push origin main
```

### 6. Sync to AUR

```bash
cd ~/git/aur-hyprscreen
cp /home/franlol/Documents/hyprscreen/pkg/PKGBUILD .
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "<type>(vX.Y.Z): <message>"
git push
```

### 7. Verify the package is live

```bash
curl -s 'https://aur.archlinux.org/rpc/v5/info?arg[]=hyprscreen' | grep -oP '"Version":"[^"]*"'
```

Expect `"Version":"X.Y.Z-1"`. May lag 1–3 minutes behind the push.

## Commit Message Style

Conventional commits, matching existing history:

- `feat:` — new user-facing feature
- `fix:` — bug fix. Version-bumping fixes get the `(vX.Y.Z)` suffix.
- `chore(pkg):` — packaging-only changes (e.g., updating `pkg/PKGBUILD` sha256)
- `docs:` — documentation only
- `chore:` — repo housekeeping

Examples from history: `fix: embed icons, self-detach from terminal, no main-window flash on CLI flow`, `chore(pkg): v0.1.2 sha256`, `fix: force Adwaita theme to silence broken-theme GTK warnings (v0.1.2)`.

## Gotchas

- **PKGBUILD references its own future tarball.** The PKGBUILD hash describes a tarball that GitHub only generates after the tag is pushed. Always tag → hash → PKGBUILD update, in that order. Don't try to push the PKGBUILD with the real hash before the tag exists.

- **`.SRCINFO` MUST be regenerated and pushed alongside the PKGBUILD bump.** AUR uses `.SRCINFO` (not `PKGBUILD`) for its package database and RPC API. A push that updates PKGBUILD but not `.SRCINFO` leaves the new version invisible to `paru` users. AUR warns about this on the push: `remote: warning: .SRCINFO unchanged. The package database will not be updated!`

- **AUR rejects non-fast-forward pushes.** History can't be rewritten on AUR. If a local `git rebase` squashed/edited commits that origin already has, force-push will be rejected. Recover with `git reset --hard origin/master` and add the new work as a separate commit on top.

- **paru caches AUR clones locally.** After a fresh push, `paru -Syu` may serve a stale clone from `~/.cache/paru/clone/hyprscreen/`. Fix with `paru -S hyprscreen --redownload` or `rm -rf ~/.cache/paru/clone/hyprscreen`. This is a paru-side issue, not an AUR issue.

- **AUR's RPC cache lags the push.** The web UI updates quickly, but the RPC endpoint (which `paru` queries) can take 1–3 minutes to refresh. Verify with the curl one-liner from step 7 before assuming a push failed.
