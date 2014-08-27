#version 140

uniform float pointScale;
uniform mat4 transformation;

in float position_x;
in float position_y;
in float position_z;
out vec4 Color;
out vec2 Position;

void main() {
    vec4 realpos = transformation * vec4(position_x, position_y, position_z, 1.0);
    float t = 1.0 / (1.0 + exp(-realpos.z * 5.0));
    Color = t * vec4(1.0, 0.27, 0.08, 1.0) + (1.0 - t) * vec4(0.5, 0.5, 0.5, 1.0);

    vec4 realpos2 = transformation * vec4(position_x, position_y, 0.0, 1.0);
    gl_Position = vec4(realpos2.x, realpos2.y, 0.0, 1.0);
    Position = vec2(gl_Position.x, gl_Position.y);

    gl_PointSize = pointScale;
}

