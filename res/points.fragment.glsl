#version 140

uniform float pointScale;
uniform float width;
uniform float height;
uniform float margin;

in vec4 Color;
in vec2 Position;
out vec4 out_color;

void main() {
    if (
               (gl_FragCoord.x < margin)
            || (gl_FragCoord.x >= width - margin)
            || (gl_FragCoord.y < margin)
            || (gl_FragCoord.y >= height - margin)) {
        discard;
    }

    float x = (Position.x + 1.0) / 2.0 * width;
    float y = (Position.y + 1.0) / 2.0 * height;
    float dx = x - gl_FragCoord.x;
    float dy = y - gl_FragCoord.y;
    float step1 = 0.5 * pointScale;
    float step0 = max(0.25 * pointScale, step1 - 2.0);
    float alpha = 1.0 - smoothstep(step0 * step0, step1 * step1, dx * dx + dy * dy);

    out_color = vec4(Color.r, Color.g, Color.b, Color.a * alpha);
}

