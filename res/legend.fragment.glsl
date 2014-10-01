#version 140

varying float z;
out vec4 out_color;

vec4 z2rgba(float z);

void main(void) {
    out_color = z2rgba(z);
}

