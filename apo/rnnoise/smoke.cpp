#include <cassert>
#include <cmath>
#include "../noise.h"

int main() {
    NoiseProcessor noise;
    assert(noise.prepare(L"weights_blob.bin", 1));
    float input[1440] = {};
    float output[1440] = {};
    for (unsigned i = 0; i < 1440; ++i)
        input[i] = 0.1f * std::sin(i * 0.1f);
    noise.process(input, output, 1440);
    for (unsigned i = 0; i < 960; ++i) assert(output[i] == 0.0f);
    for (float sample : output) assert(std::isfinite(sample));
    float peak = 0.0f;
    for (unsigned i = 960; i < 1440; ++i)
        if (std::fabs(output[i]) > peak) peak = std::fabs(output[i]);
    assert(peak > 0.000001f);
    for (int pass = 0; pass < 20; ++pass) {
        noise.process(input, output, 1440);
        for (float sample : output) assert(std::isfinite(sample));
    }
    noise.reset();
}
