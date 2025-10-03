#version 450

layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8_snorm) uniform image2D io_texture;

void main() {
    uint x = gl_GlobalInvocationID.x;
    uint y = gl_GlobalInvocationID.y;

    imageStore(io_texture, ivec2(x, y), vec4(0.1, 0.2, 0.3, 0.4));
}
