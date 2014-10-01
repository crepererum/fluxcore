#version 140

// source: http://lolengine.net/blog/2013/07/27/rgb-to-hsv-in-glsl
vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

vec4 z2rgba(float z) {
    float t = 1.0 / (1.0 + exp(-z * 3.0)); // use sigmoid function to limit z to (0,1)
    return vec4(hsv2rgb(vec3(0.5 - 0.5 * t, 1.0, 1.0)), 1.0);
}

