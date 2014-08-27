#version 140

uniform float count;
uniform float alpha;
uniform sampler2D fbo_texture;
varying vec2 f_texcoord;
out vec4 out_color;

void main(void) {
    vec4 tex = texture2D(fbo_texture, f_texcoord);
    if (tex.a == 0.0) {
        discard;
    } else {
        vec4 full = tex / tex.a;
        float transp = log(1.0 + pow(tex.a / count, 1.0 / alpha - 1.0)) / log(2.0);
        out_color = vec4(full.r, full.g, full.b, transp);
    }
}

