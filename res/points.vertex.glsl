#version 140

uniform float pointScale;
uniform mat4 transformation;

in float position_x;
in float position_y;
in float position_z;
out vec4 Color;
out vec2 Position;

vec4 z2rgba(float z);

void main() {
    vec4 realpos = transformation * vec4(position_x, position_y, position_z, 1.0);
    Color = z2rgba(-realpos.z);

    vec4 realpos2 = transformation * vec4(position_x, position_y, 0.0, 1.0);
    gl_Position = vec4(realpos2.x, realpos2.y, 0.0, 1.0);
    Position = vec2(gl_Position.x, gl_Position.y);

    gl_PointSize = pointScale;
}

