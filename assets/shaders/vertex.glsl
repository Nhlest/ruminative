#version 450

layout(location = 0) in vec2 in_coord;
layout(location = 1) in uint tile;

layout(constant_id = 0) const uint tile_size_x = 10;
layout(constant_id = 1) const uint tile_size_y = 10;

layout(location = 0) out vec2 tex_coords;

void main() {
  float x = gl_VertexIndex % 2;
  float y = gl_VertexIndex / 2;
  gl_Position = vec4(in_coord.x + x * 0.1, in_coord.y + y * 0.1, 0.0, 1.0);
  uint tile_x = tile % tile_size_x;
  uint tile_y = tile / tile_size_x;
  tex_coords = vec2((x + tile_x) / tile_size_x, (y + tile_y) / tile_size_y);
}