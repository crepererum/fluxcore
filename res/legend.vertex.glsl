#version 140

attribute vec2 v_coord;
uniform float width;
uniform float height;
uniform float margin;
varying float z;

void main(void) {
    gl_Position = vec4(v_coord.x * (width - 2.0 * margin) / width, v_coord.y * (margin / 5.0) / height - (height - margin / 5.0) / height, 0.0, 1.0);
    z = v_coord.x;
}

