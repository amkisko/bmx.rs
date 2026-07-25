# Release binary size

## Participants

- amkisko

## Decisions

- Add workspace `[profile.release]` with thin LTO, codegen-units = 1, strip, and panic = abort.

## Effects

- Release builds strip symbols and apply thin LTO by default.
- Measured arm64 macOS release size: 5.0 MB before, 3.1 MB after (39%).

## Next

- Optional: further dependency feature trimming if install-time size still matters.

## Source

- Binary size analysis session (timely, scout, status, pray, bmx)
