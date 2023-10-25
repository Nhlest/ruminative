#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D tex;

void main() {
  vec4 tex = texture(tex, tex_coords);
  if (tex.rgb == vec3(0.0, 0.0, 0.0)) {
    tex.a = 0.0;
  }
  f_color = tex;
}