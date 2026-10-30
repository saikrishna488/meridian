# Third-party software

## wayland-pipewire-idle-inhibit

Meridian builds `wayland-pipewire-idle-inhibit` version 0.7.1 from its pinned
crates.io release as part of `tools/compile_meridian.sh`, then installs it with
`tools/install.sh`. It is licensed under GPL-3.0. See the
[upstream repository](https://github.com/rafaelrc7/wayland-pipewire-idle-inhibit)
and the crate's published license and source for full terms.

## Glisse

Meridian uses [Glisse](https://github.com/0kzh/glisse) 0.4.3 for React UI
components and glass materials. Glisse is copyright 2026 Kelvin Zhang and is
licensed under the MIT license; the complete license is distributed in the
`ui/src/vendor/GLISSE-LICENSE.txt` and the `glisse` npm package. Its peer dependencies are React and React DOM.
