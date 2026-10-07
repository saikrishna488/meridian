# Terminal renderer

xterm.js 5.5.0 and FitAddon 0.10.0, MIT licensed. Included locally so builds
and the desktop work offline. Keep LICENSE alongside these files.

Upstream: https://github.com/xtermjs/xterm.js/tree/5.5.0

The unmodified npm UMD builds were obtained from
https://github.com/protoLabsAI/terminal-plugin/tree/main/vendor:

- xterm.js Git blob: 7ca75f8e5ec2a0741333130fd63fb9c79d9c2023
- addon-fit.js Git blob: 038897193db54dd0f09783d53e602755b93a2eaf
- xterm.css Git blob: e97b6439055da076119d8ce42225944623b80e9b

To update, use the official @xterm/xterm and @xterm/addon-fit npm packages,
copy their lib/*.js and css/xterm.css, then update this manifest and license.
No CDN, websocket listener, or externally loaded scripts are used.
