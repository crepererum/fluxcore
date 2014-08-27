#version 140

uniform float pointScale;
uniform mat4 transformation;

in float position_x;
in float position_y;
in float position_z;
out vec4 Color;
out vec2 Position;

// source: http://lolengine.net/blog/2013/07/27/rgb-to-hsv-in-glsl
vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

void main() {
    vec4 realpos = transformation * vec4(position_x, position_y, position_z, 1.0);
    float t = 1.0 / (1.0 + exp(-realpos.z * 3.0)); // use sigma function to limit z to (0,1)
    Color = vec4(hsv2rgb(vec3(0.5 - 0.5 * t, 1.0, 1.0)), 1.0);

    vec4 realpos2 = transformation * vec4(position_x, position_y, 0.0, 1.0);
    gl_Position = vec4(realpos2.x, realpos2.y, 0.0, 1.0);
    Position = vec2(gl_Position.x, gl_Position.y);

    gl_PointSize = pointScale;
}

