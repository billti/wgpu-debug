@group(0) @binding(0)
var<storage, read_write> stateVec: array<u32>;

@compute @workgroup_size(8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    let old_value = stateVec[index];
    stateVec[index] = index + old_value;
}