#pragma once

#include <cstdio>
#include <new>
#include "rnnoise.h"

class NoiseProcessor {
    static constexpr unsigned frameSize = 480;
    struct Channel {
        DenoiseState* state = nullptr;
        float input[frameSize] = {};
        float output[frameSize] = {};
        float dry[frameSize] = {};
    };

    RNNModel* model = nullptr;
    FILE* modelFile = nullptr;
    Channel* channels = nullptr;
    unsigned channelCount = 0;
    unsigned position = 0;
    float strength = 1.0f;

public:
    NoiseProcessor() = default;
    NoiseProcessor(const NoiseProcessor&) = delete;
    NoiseProcessor& operator=(const NoiseProcessor&) = delete;
    ~NoiseProcessor() { reset(); }

    bool active() const { return channels != nullptr; }

    bool prepare(const wchar_t* modelPath, unsigned count, float mix = 1.0f) {
        reset();
        if (count == 0) return false;
        strength = mix;
        if (_wfopen_s(&modelFile, modelPath, L"rb") != 0 || !modelFile) return false;
        model = rnnoise_model_from_file(modelFile);
        if (!model) { reset(); return false; }
        channels = new (std::nothrow) Channel[count]();
        if (!channels) { reset(); return false; }
        channelCount = count;
        for (unsigned i = 0; i < count; ++i) {
            channels[i].state = rnnoise_create(model);
            if (!channels[i].state) { reset(); return false; }
        }
        return true;
    }

    void reset() {
        if (channels) {
            for (unsigned i = 0; i < channelCount; ++i)
                if (channels[i].state) rnnoise_destroy(channels[i].state);
            delete[] channels;
        }
        channels = nullptr;
        channelCount = position = 0;
        if (model) rnnoise_model_free(model);
        model = nullptr;
        if (modelFile) fclose(modelFile);
        modelFile = nullptr;
    }

    void process(const float* input, float* output, unsigned frames) {
        for (unsigned frame = 0; frame < frames; ++frame) {
            for (unsigned channel = 0; channel < channelCount; ++channel) {
                const unsigned index = frame * channelCount + channel;
                const float sample = input[index];
                output[index] = strength * channels[channel].output[position] / 32768.0f +
                    (1.0f - strength) * channels[channel].dry[position];
                channels[channel].dry[position] = sample;
                channels[channel].input[position] = sample * 32768.0f;
            }
            if (++position == frameSize) {
                for (unsigned channel = 0; channel < channelCount; ++channel)
                    rnnoise_process_frame(channels[channel].state, channels[channel].output, channels[channel].input);
                position = 0;
            }
        }
    }
};
