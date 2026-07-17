# Release Process

SleekManualMaker releases are built and published by GitHub Actions.

## Automatic Release

Create and push a `v*` tag:

```bash
git tag v2.0.1
git push origin v2.0.1
```

The `Release` workflow will:

- run formatting, check, test, and clippy on `windows-latest`;
- build `SleekManualMaker.exe` in release mode;
- create `SleekManualMaker.zip`;
- create `SleekManualMaker.zip.sha256`;
- create or update the GitHub Release for that tag.

## Manual Rebuild

Open GitHub Actions, run the `Release` workflow manually, and provide the tag name.

Use this when a release asset needs to be rebuilt for an existing tag.

## Local Package Build

```powershell
.\scripts\build_release_zip.ps1
```

This creates `SleekManualMaker.zip` and prints its SHA-256 hash.
