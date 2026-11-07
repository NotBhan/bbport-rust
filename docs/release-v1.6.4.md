## Bloodborne for Windows v1.6.4

Download **Bloodborne-Windows-v1.6.4-Jgamrot.zip** for a first installation. It includes the
launcher, game executable, DLSS runtime, and Xbox button prompts. You still need your own
extracted CUSA03173 1.09 game folder; game files are not included. Follow the [first-time
setup guide](https://github.com/Jgamrot/bloodborne_pc#first-time-setup).

### New

- The port now records trophy unlock events in `user\trophies.txt`, shows a six-second popup,
  and lists earned trophies under **Achievements (local)** in the Insert/L3+R3 menu.
- Trophies earned before this build cannot be reconstructed from old saves. The game's
  trophy pack is encrypted, so entries display trophy IDs rather than names for now. These
  achievements are local and do not sync to PlayStation Network.
- The launcher now correctly identifies itself as v1.6.4, and the full package includes
  the Tk files needed by its Windows interface.

The v1.6.3 runtime changes remain included: shorter death respawns on supported 1.09 images,
less duplicate shader preload work, and avoidance of repeated GPU waits when motion-vector
render sizes change. This release does not claim a new measured FPS gain.

For an existing installation, close the game and replace `bin\bb-probe.exe` with the separate
asset. Your saves, settings, and mods remain in place. Use the full zip to update the launcher.
