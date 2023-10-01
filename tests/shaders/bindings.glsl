#version 450

layout(local_size_x = 256) in;

layout(std430, set = 0, binding = 0) buffer _x0 {
    uint data0[];
};

layout(std430, set = 0, binding = 1) buffer _x1 {
    uint data1[];
};

layout(std430, set = 0, binding = 2) buffer _x2 {
    uint data2[];
};


layout(std430, set = 0, binding = 3) buffer _x3 {
    float my_length;
    uint data3[];
};

layout(set = 0, binding = 4) uniform sampler2D input_texture;

layout(set = 0, binding = 5, rgba8) uniform image2D input_output_texture[4];

layout(set = 0, binding = 6) uniform BufferObject {
    mat4 someMatrix;
};

layout(push_constant) uniform PushConstants {
    mat4 modelViewProjection;
    vec4 color;
} pc;


void main() {
    uint idx = gl_GlobalInvocationID.x;
    data0[idx] = data1[idx] + data2[idx];
}