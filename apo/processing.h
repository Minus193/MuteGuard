#pragma once

inline void attenuate(const float* input, float* output, unsigned sampleCount)
{
    for (unsigned i = 0; i < sampleCount; ++i)
        output[i] = input[i] * 0.5f;
}
