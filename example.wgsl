struct InstanceData {
   @location(0) value_a: f32,
   @location(1) value_b: f32,
   @location(2) value_c: f32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>
}

@vertex
fn vertex_main(
    data: InstanceData
) -> VertexOutput {
    var output: VertexOutput;

    output.pos = vec4(
        data.value_a,
        data.value_b,
        data.value_c,
        1.0,
    );

    return output;
}
