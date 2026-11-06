# Project instructions

- Whenever adding a feature, user-visible improvement, or relevant bug fix, update the root `features.md` in the same change.
- Group entries by the date added, using `YYYY-MM-DD`. Describe the actual behavior and any material limitations; preserve existing entries.
- Keep machine-specific administration changes (such as local swap or zram configuration) out of the Meridian feature list unless implemented as a Meridian product feature.
- Do not run automated tests or start Node-based verification processes unless the user explicitly requests them again. Review source code statically and report verification limits; the user's 12 GB machine cannot accommodate the test processes' memory usage.
