# Luminare adaptation

MarsOSKit adapts the surface, button-state and grouped-section design architecture
of [Luminare](https://github.com/MrKai77/Luminare), revision
`7958d31278ad054abf0a7cf19e5ae700399a812f`. Reference components:
`LuminareSurfaceStyle`, `LuminareSurfaceButtonStyle`, and `LuminareSection`.
The upstream BSD-3-Clause notice is retained in `Licenses/Luminare-BSD-3-Clause.md`.

The SwiftCrossUI views and C++ GTK stylesheet are a Linux implementation, not a
binary-compatible port of Apple's SwiftUI. AppKit visual effects, Apple fonts,
SF Symbols, window management and accessibility APIs are not copied. Application
artwork comes from the user's installed Linux icon theme. Materials use alpha
compositing with a solid fallback; compositor background blur is not implemented.
SwiftCrossUI's separate MIT license remains in `LICENSE.SwiftCrossUI`.
