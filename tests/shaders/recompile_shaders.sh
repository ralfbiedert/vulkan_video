args="-fshader-stage=compute -O"
glslc $args ./hello_world.glsl -o ./compiled/hello_world.spv
glslc $args ./bindings.glsl -o ./compiled/bindings.spv
glslc $args ./image_color.glsl -o ./compiled/image_color.spv
