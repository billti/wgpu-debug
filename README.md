# wgpu-debug

This repo demonstrates an issue trying to use the Xcode GPU debugger with [wgpu](https://github.com/gfx-rs/wgpu) buffers initialized with data
using `create_buffer_init`.

## Repro Steps

On line 5 of `src/main.rs`, set `USE_MAPPABLE_BUFFERS` to `false`. If you build and run this under Xcode with `MTL_CAPTURE_ENABLED=1`,
you will see that the GPU debugger does not see the initial data in the buffer, just zeroes. This subsequently breaks the GPU debugger
in Xcode as the data is not available for inspection or replay.

If you set it to `true`, it will work. By using a mappable buffer, the data is available to the GPU debugger,
and the GPU debugger works as expected.

## Notes

The underlying issue appears to be in the branch at <https://github.com/gfx-rs/wgpu/blob/v26/wgpu-core/src/device/resource.rs#L742>,
where if `MAP_WRITE` is not present, it takes the branch at line 761 that creates a staging buffer. Xcode doesn't seem to handle this
staging buffer flow for some reason.
