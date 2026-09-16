# Release media

Captured on 2026-09-16 from commit `92fabeb`, using published Slint 1.18.0
on macOS with the Winit/Skia backend. No application code was changed for capture.

- `advanced.png`: full-resolution MCP window screenshot (2000 × 1600).
  The advanced example has two additional connections created through pin drags.
- `automatic-layout.png`: full-resolution MCP window screenshot (2000 × 1400),
  after invoking the example's Auto Layout button.
- `demo.mp4`: 32-second silent H.264 video, 1280 × 1120, with chapter captions.
  The advanced scene shows node dragging, connection creation and embedded
  controls; the second scene shows Scramble and Auto Layout.

## Capture method

Launch the examples separately with Slint's embedded MCP server:

```sh
SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=9315 cargo run -p advanced --features slint/mcp
SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=9316 cargo run -p sugiyama --features slint/mcp
```

Connect to each local `/mcp` endpoint. Discover window and element handles with
`list_windows`, `get_window_properties`, and `get_element_tree`. Drive the UI with
`click_element` and `drag_element`; collect PNG frames with `take_screenshot`.
Handles belong to the running session and must be discovered again on restart.

The still images are original MCP captures. The video is assembled from MCP
PNG frames using FFmpeg, with pauses shortened and frame timing adjusted for
presentation. Captions sit below the application image; the layout scene is
letterboxed to match the advanced window's aspect ratio. It is a feature demo,
not a frame-rate or performance recording. The encoded stream is 24 fps, with
repeated frames between captured snapshots and no synthesized motion.

The README links the repository MP4 instead of relying on an external attachment.
For an inline GitHub video player, upload `demo.mp4` through GitHub's Markdown
editor and replace the MP4 link with the resulting attachment URL.
