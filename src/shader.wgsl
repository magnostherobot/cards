struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) rank: u32,
    @location(10) suit: u32,
    @location(11) facedown: u32,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) rank: u32,
    @location(2) suit: u32,
    @location(3) facedown: u32,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let instance_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * instance_matrix * vec4<f32>(model.position, 1.0);
    out.rank = instance.rank;
    out.suit = instance.suit;
    out.facedown = instance.facedown;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let cards_per_row = 13.0;
    let cards_per_col = 5.0;
    let tex_size = vec2(cards_per_row, cards_per_col);

    let facedown_tex_tl = vec2(0.0, 4.0);
    let faceup_tex_tl = vec2(f32(in.rank), f32(in.suit));

    let coords = (select(faceup_tex_tl, facedown_tex_tl, bool(in.facedown)) + in.tex_coords) / tex_size;

    return textureSample(t_diffuse, s_diffuse, coords);
}
