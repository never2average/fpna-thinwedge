# Public History Sanitization

The current tree is scanned by `scripts/check_public_release_readiness.py`. Before making the repository public, also scan reachable git history:

```bash
python3 scripts/check_public_history_readiness.py
```

This scanner prints counts and object/path samples only. It intentionally does not print matched secret values.

If the scan fails, choose one path before public visibility is enabled:

1. Rewrite history or publish from a sanitized history branch, then rerun the scanner until it passes.
2. Explicitly accept the inherited-history exposure in the release record.

Current status: the public history rewrite has removed the historical test private-key PEM fixtures and fake/test `sk-...` strings from reachable branches and tags. No live CircleCI, GitHub, npm, AWS, Slack, Google, or Stripe token-shaped values were found in the latest manual scan. Keep this script as the repeatable repo-local gate for future checks.

After any future history rewrite, verify release refs again:

```bash
git fetch --all --tags --force
python3 scripts/check_public_history_readiness.py
python3 scripts/check_public_release_readiness.py
```
