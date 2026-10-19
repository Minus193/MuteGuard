#pragma once
#include <algorithm>
#include <cmath>
#include <vector>

// Bounded stereo queue: discard old frames on overflow instead of accumulating delay.
struct MonitorBuffer {
    std::vector<float> samples;
    size_t head = 0, count = 0;
    explicit MonitorBuffer(unsigned frames) : samples(size_t(frames) * 2) {}
    void push(const float* input, unsigned frames, unsigned channels) {
        const size_t capacity = samples.size() / 2;
        for (unsigned frame = 0; frame < frames; ++frame) {
            if (count == capacity) { head = (head + 1) % capacity; --count; }
            const size_t tail = (head + count) % capacity;
            for (unsigned channel = 0; channel < 2; ++channel) {
                const float value = input[size_t(frame) * channels + (channels == 1 ? 0 : channel)];
                samples[tail * 2 + channel] = std::isfinite(value) ? (std::max)(-1.0f, (std::min)(1.0f, value)) : 0;
            }
            ++count;
        }
    }
    void pop(float* output, unsigned frames) {
        std::fill_n(output, size_t(frames) * 2, 0.0f);
        for (unsigned frame = 0; frame < frames && count; ++frame) {
            output[frame * 2] = samples[head * 2];
            output[frame * 2 + 1] = samples[head * 2 + 1];
            head = (head + 1) % (samples.size() / 2); --count;
        }
    }
};
