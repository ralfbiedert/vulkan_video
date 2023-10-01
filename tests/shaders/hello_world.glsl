#version 450


layout(local_size_x = 32, local_size_y = 1, local_size_z = 1) in;


layout(std430, set = 0, binding = 0) buffer _x0 {
    uint data0[];
};

layout(std430, set = 0, binding = 1) buffer _x1 {
    uint data1[];
};

layout(std430, set = 0, binding = 2) buffer _x2 {
    uint data2[];
};

void main() {
    uint idx = gl_GlobalInvocationID.x;
    data0[idx] = data1[idx] + data2[idx];
}