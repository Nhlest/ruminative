#version 450

layout(location = 0) in vec2 in_coord;
layout(location = 1) in uvec2 tile;

//layout(constant_id = 0) const uint tile_size_x = 10;
//layout(constant_id = 1) const uint tile_size_y = 10;
//
layout(push_constant) uniform Constants {
  vec2 offset;
  uint tile_size_x;
  uint tile_size_y;
} push;

layout(location = 0) out vec2 tex_coords;

void main() {
  float x = gl_VertexIndex % 2;
  float y = gl_VertexIndex / 2;
  gl_Position = vec4(in_coord.x + x * 0.1, in_coord.y + y * 0.1, 0.0, 1.0) + vec4(push.offset, 0.0, 0.0);
  uint tile_x = tile.x;
  uint tile_y = tile.y;
  tex_coords = vec2((x + tile_x) / push.tile_size_x, (y + tile_y) / push.tile_size_y);
}