#pragma once
#include <algorithm>

// Versioned by size at the DLL boundary; mirrored by the Rust analyzer.
struct PreviewSnapshot {
    unsigned size = sizeof(PreviewSnapshot);
    int status = 0;
    HRESULT error = S_OK;
    unsigned rate = 0, channels = 0, sequence = 0, revision = 0, noiseActive = 0;
    float inputPeak = -100, outputPeak = -100;
    float input[64], output[64];
    int monitorStatus = 0;
    HRESULT monitorError = S_OK;
    PreviewSnapshot() { std::fill_n(input, 64, -100.0f); std::fill_n(output, 64, -100.0f); }
};
static_assert(sizeof(PreviewSnapshot) == 560, "Analyzer ABI changed");
