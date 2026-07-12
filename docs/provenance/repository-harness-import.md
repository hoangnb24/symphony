# Repository Harness Import Provenance

Symphony was extracted from `hoangnb24/repository-harness` with filtered Git
history. This record describes the immutable import boundary; later standalone
workspace changes begin after the raw-import tag.

## Immutable Anchors

- Source repository: `git@github.com:hoangnb24/repository-harness.git`
- Accepted source commit: `6e8243f2a5cb6a32cf0a7a0ecebdb257a429bdd9`
- Source recovery tag: `pre-symphony-extraction-20260711`
- Source bundle SHA-256:
  `cc6b868567750e139d167e8b674d8016359e0e8c66307446ef15fe6ae4df712d`
- Approved path manifest SHA-256:
  `e949ed330ace1e6ae80aa0bbe737dce831732d18bef62edf288eb00f8de876cf`
- Filter commit-map SHA-256:
  `eb4d02e507e6fe0f110a20b1acad39b361e1cc7aa0a48d34028a2f5f31e81238`
- Raw filtered source HEAD before this provenance commit:
  `4bd3acb73659a4a188a18b5306374495e75302a5`
- Extraction date: `2026-07-12`

## Pinned Filter Tool

- Upstream: `https://github.com/newren/git-filter-repo`
- Release: `v2.47.0`
- Tag object: `cbad6503f5de690c9d5a376d900136691c330793`
- Tag commit: `6f79afc8c90c592a3052e6cc53c2ca8907515bca`
- Reported tool version: `a40bce548d2c`
- Release archive SHA-256:
  `8de0b87d3e8137b5af394d4d0e0c6faa05a14375f3e3edea704d4255be039cd3`

The tool archive was downloaded from the upstream GitHub tag URL, hashed
before use, and executed only against an external disposable clone.

## Filter Procedure

The recovery bundle was cloned externally. Every imported local, remote, and
tag ref was deleted from the filtering namespace. A single branch named
`extraction` was created at the accepted source commit, then filtered with:

```text
git-filter-repo v2.47.0 --force \
  --paths-from-file .git/e11-paths-from-file.txt \
  --source <disposable-clone> \
  --target <disposable-clone>
```

The exact 100 literal paths are in `e11-filter-paths.txt`. The upstream
old-to-new mapping for filtered source commits is in
`e11-filter-repo-commit-map.txt`. Representative lineage includes:

| Source commit | Filtered commit | Purpose |
| --- | --- | --- |
| `e7a124b763d5ab9dc6a9e6edfb5afff0867a7353` | `8d4834f4f48fec5d5bfa97c144a7c5c88871fe40` | Accepted Symphony scope lineage |
| `444d793f17f3a4f59161a0bd15ec590c500c0150` | `6de52406378dd46dad806faf2ac2750c85b8c44b` | Local runner implementation |
| `f539f5ded7479ffc932555977282c9c22e432746` | `64cf6efc33d1e9c8447bcfb84744bf6893d1c9e1` | Web UI implementation |
| `6e8243f2a5cb6a32cf0a7a0ecebdb257a429bdd9` | `4bd3acb73659a4a188a18b5306374495e75302a5` | Frozen source boundary |

## Intentional Exclusions

The import excludes Harness CLI source and schemas, root workspace/lock files,
live databases and changesets, generated run/worktree state, and project-local
`.agents`, `.codex`, and `.impeccable` tooling. Root standalone files are not
missing migration inputs: US-091 creates product-owned workspace metadata after
this raw history boundary.

## Authorized First Push

The first target write is restricted to exactly these refspecs after a fresh
owner go/no-go and a final empty-remote check:

```text
HEAD:refs/heads/main
refs/tags/symphony-raw-import-20260712:refs/tags/symphony-raw-import-20260712
```

`--mirror`, `--all`, force push, and recovery-bundle refs are forbidden.
