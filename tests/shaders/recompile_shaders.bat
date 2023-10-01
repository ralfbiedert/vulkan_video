@echo off

echo.
echo Compiling shaders ...
echo.

set args=-fshader-stage=compute -O
REM set args=-fshader-stage=compute -O --target-env=vulkan1.3 --- gives error 'maintenance4' not enabled when running shader

glslc %args% .\hello_world.glsl -o .\compiled\hello_world.spv
glslc %args% .\bindings.glsl -o .\compiled\bindings.spv
glslc %args% .\image_color.glsl -o .\compiled\image_color.spv

echo.
echo Done.
echo.

pause