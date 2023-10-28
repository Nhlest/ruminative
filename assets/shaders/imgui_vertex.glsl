#version 450

layout(push_constant) uniform PushConstants {
  float window_width;
  float window_height;
} push_constants;

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec4 col;

layout(location = 0) out vec2 uv_out;
layout(location = 1) out vec4 col_out;

void main() {
  uv_out = uv;
  col_out = col;
  gl_Position = vec4(pos.x / (push_constants.window_width / 2.0) - 1.0, pos.y / (push_constants.window_height / 2.0) - 1.0, 0.0, 1.0);
}