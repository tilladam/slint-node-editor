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
recurred: 2026-09-17 The user reported the crates.io link was still broken after the repository README fix. The crates.io 1.0.0 README endpoint confirmed the original attachment URL remained embedded. Repairing main alone was insufficient; publish a documentation patch and verify the README served by crates.io.

## 2026-09-18 · Fixed window dimensions invalidate native resize probes · #integration-tests #slint #viewport
Situation: Reproducing issue #6 with Slint 1.18.0 and the headless integration-test window.
Tried: Dispatched a native resize event and then used Window::set_size while the fixture declared width: 800px and height: 600px.
Outcome: The editor stayed 800 by 600 and no resize notification fired. Generated bindings for the window's exposed dimensions and editor size were constants, so the test was not exercising a resizable viewport.
Evidence: The failing viewport_resize test printed unchanged window/editor sizes; generated test.rs initialized those properties with literal 800 and 600 values.
Next time: Verified alternative: use preferred-width/preferred-height in a resizable fixture, resize through Window::set_size, and assert the actual dimensions before checking the callback result.
Revisit when: Testing a deliberately fixed-size window or a Slint version with different sizing semantics.
Cost: Two additional failing test runs and generated-code inspection.
Scope: project     Status: active

## 2026-09-18 · Checking only the last callback misses notification semantics · #viewport #integration-tests #api
Situation: PR #8 introduced viewport-resized for hosts maintaining their visible bounds.
Tried: Asserted the last callback dimensions after resizing and documented connecting a handler, without checking the number of notifications or initial state.
Outcome: The Claude review and a runtime probe found two identical callbacks for a combined width/height resize and no initial notification, including for a handler connected before show().
Evidence: The probe recorded [(1000.0, 700.0), (1000.0, 700.0)] for one resize and [] on initial subscription.
Next time: Verify the full event sequence for batched changes and explicitly test/document how a new subscriber obtains its initial state. The fix coalesces size changes and documents host seeding.
Revisit when: An API explicitly requires per-property events or provides a replaying subscription mechanism.
Cost: Adversarial-review follow-up and additional regression coverage.
Scope: project     Status: active
