# bummer.md — what did not work here and why.
# Append-only during tasks; the `bummer` skill merges, promotes and retires.
# Template per entry:
#
# ## YYYY-MM-DD · title · #tags/paths
# Situation: task, constraints, versions
# Tried: what seemed reasonable / what was proposed
# Outcome: what failed or was insufficient (observed, not inferred)
# Evidence: dated observation or user correction, reference if any
# Hypothesis: suspected cause (optional; keep separate from Outcome)
# Next time: verified alternative, or suggestion — say which
# Revisit when: conditions that would invalidate this
# Cost: what the dead end cost (time, reverts, user corrections)
# Scope: project | global-candidate     Status: active
# recurred: YYYY-MM-DD note   (append to the existing entry, no new entry)

## 2026-09-17 · Preview does not verify attachment availability · #README.md #docs/media #github #release
Situation: Publishing slint-node-editor 1.0.0 with a video in the README.
Tried: Used a GitHub user-attachment URL after checking the inline Markdown preview.
Outcome: The user reported a 404 after publication; an unauthenticated download confirmed it. The repository MP4 still returned HTTP 200.
Evidence: 2026-09-17 user report and HTTP checks for attachment 0c3e6c2f-ce5d-4dfb-b2df-67c7bdc611fc and the tagged repository MP4.
Next time: Verify media URLs without authentication before publication. A versioned GitHub release asset is an alternative for the download link; verify its contents against the committed MP4.
Revisit when: A replacement attachment is publicly accessible and its persistence has been verified independently of an editor preview.
Cost: User correction and post-release documentation repair; the published 1.0.0 crate still contains the original README.
Scope: project     Status: active
