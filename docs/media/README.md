# Release media

Captured on 2026-09-16 using published Slint 1.18.0 on macOS with the Winit/Skia
backend. The stills come from commit `92fabeb`; the replacement video comes from
`2631e4d`. No application code was changed for capture.

- `advanced.png`: original MCP window screenshot (2000 × 1600).
  The advanced example has two additional connections created through pin drags.
- `automatic-layout.png`: original MCP window screenshot (2000 × 1400),
  after invoking the example's Auto Layout button.
- `demo.mp4`: 32.9-second silent H.264 video, 1000 × 880, 30 fps, with chapter
  captions. Playback preserves the capture timing; there is no speed-up or
  synthesized motion.

## Video chapters

- 0:00–0:13: node dragging, connection creation, embedded controls, and zoom.
- 0:13–0:25: four animated connections growing into place with glow effects,
  followed by moving a connected node.
- 0:25–0:33: Scramble and Auto Layout in the Sugiyama example.

## Capture method

Build with debug metadata and optimized image encoding. The capture-only Cargo
profile keeps MCP screenshots fast without changing the application's source
or the project's normal build configuration:

```sh
SLINT_EMIT_DEBUG_INFO=1 cargo build --config docs/media/capture-profile.toml \
    -p advanced -p animated-links -p sugiyama --features slint/mcp
```

Launch each binary separately. Use 1× scale to capture native 1000-pixel-wide
frames instead of encoding Retina-sized PNGs on every frame:

```sh
SLINT_SCALE_FACTOR=1 SLINT_MCP_PORT=9315 target/debug/advanced
SLINT_SCALE_FACTOR=1 SLINT_MCP_PORT=9317 target/debug/animated-links
SLINT_SCALE_FACTOR=1 SLINT_MCP_PORT=9316 target/debug/sugiyama
```

Connect to each local `/mcp` endpoint. Discover window and element handles with
`list_windows`, `get_window_properties`, and `get_element_tree`. Drive the UI with
`click_element`, `drag_element`, and `dispatch_pointer_scroll`. Handles belong to
the running session and must be discovered again on restart.

Poll `take_screenshot` at 30 Hz in a separate client thread while driving the
interactions. Save a monotonic timestamp for each frame. Assemble the PNGs with
FFmpeg's concat demuxer, using timestamp differences as frame durations and
`option framerate 1000` for millisecond input time-base resolution, then encode
at 30 fps. Merely assigning a high output frame rate to sparse screenshots does
not make a smooth recording.

### Capture verification

| Scene | Captured frames | Measured capture fps | Longest frame interval |
|---|---:|---:|---:|
| Advanced | 398 | 30.00 | 37.5 ms |
| Animated links | 364 | 29.99 | 37.6 ms |
| Automatic layout | 222 | 29.99 | 38.3 ms |

Each animated-link creation had 30–31 distinct PNG frames in its first second.
The complete MP4 decodes without errors and contains 987 output frames at 30 fps.
These numbers verify capture cadence, not general application performance.

Captions sit below the application image. The animated-links and layout scenes
are letterboxed to match the advanced window's aspect ratio. The still images
retain their original full resolution.

The main README embeds the video through this GitHub attachment URL:

https://github.com/user-attachments/assets/0c3e6c2f-ce5d-4dfb-b2df-67c7bdc611fc

GitHub's Markdown preview was checked for the inline video player. The repository
MP4 remains available as a download. When replacing the video, upload the new
`demo.mp4` through GitHub's Markdown attachment control and update the bare URL
in the main README as well as this record. No issue or comment needs to be posted.
