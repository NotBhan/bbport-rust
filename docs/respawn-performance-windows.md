# Windows respawn and performance improvements

These changes adapt [AwesomeObserver's respawn analysis](https://github.com/AwesomeObserver/bloodborne_pc/blob/windows/docs/respawn-loading-windows.md) and [performance audit](https://github.com/AwesomeObserver/bloodborne_pc/blob/windows/docs/performance-windows.md) to this fork. Credit for the original respawn patch and analysis goes to Evgeniy Doronin.

## Faster death respawns

Bloodborne 1.09 sets a 12-second minimum loading countdown after a player death. The loader replaces only the instruction that loads that minimum with a zeroing instruction. The game's character readiness, asset loading, and separate timeout still run. Actual loading can therefore take longer than zero seconds.

Two tested linked-image layouts place the same instructions at different offsets. The loader checks the surrounding initialization block, the readiness branches, and the 12.0 constant before changing eight bytes. If any check fails, it logs a skip and leaves the image alone. No game files on disk are modified.

The fix is enabled by default. To compare the original behavior, add `BB_RESPAWN_DELAY_FIX=0` in the launcher's **Advanced → environment overrides** and restart the game. Remove that override to re-enable it.

## Startup and shader cache

Shader warmup now creates the Vulkan pipeline cache handle before loading pipelines. It also checks for an already loaded shader module before reopening that module's SPIR-V file. The aim is to reduce repeat work during cache warmup. We have not measured a gameplay FPS gain, and first visits to new areas can still stutter while shaders compile.

The AMD-only experimental memory changes in another fork were not applied to this NVIDIA-focused build. Existing shader cache profile checks already rebuild mismatched cache entries.

## Water-area GPU stalls

[Issue #30](https://github.com/deadinside28/bloodborne_pc/issues/30) traced a severe slowdown in the Yharnam sewer water to `ObjectMotion::EnsureImage()` recreating a motion-vector image and calling `Scheduler::Finish()` whenever draw passes switched render sizes. The reporter measured about 40 FPS with object motion on and about 67 FPS with it off; a second user confirmed that the upstream fix improved water rooms. Those measurements are from their machines, not this Windows build.

We adapted the current [deadinside28 implementation](https://github.com/deadinside28/bloodborne_pc), which keeps up to four images keyed by render size. Replaced images are retired when the GPU has finished their work, without a full GPU drain. The Windows port keeps its direct command-buffer barrier in `PrepareRead()`. We built and started the game with object motion enabled; a controlled in-water FPS comparison remains to be done here.

## Validation

- The Windows respawn regression passed with both supported layouts and this installation's prepared CUSA03173 1.09 image. It executes the original and patched initialization instructions, checks the readiness branches and 15-second timeout, rejects modified or short images, and checks idempotence.
- The updated Windows executable built successfully and reached guest startup with `Respawn loading: 12-second minimum delay removed; readiness checks preserved` in its log. A short isolated-save launch also initialized the DLSS bridge.
- The motion-history regression passed. A short isolated-save launch with the new motion-image cache reached guest startup with object motion and DLSS enabled. The camera-motion test cannot compile with this Windows test stub because MSYS2 lacks `sigjmp_buf`.
- A timed before/after death respawn and gameplay FPS capture have not yet been made on this fork.
