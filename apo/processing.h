#pragma once

#include <cmath>

struct Effects {
    // shortcut: fixed parameters match this microphone; expose tuning if more setups need it.
    bool gate = false;
    bool compressor = false;
    float gateEnvelope = 0.0f;
    float gateGain = 0.0f;
    float compressorEnvelope = 0.0f;
    unsigned gateHold = 0;
    unsigned holdFrames = 0;
    float gateAttack = 0.0f;
    float gateRelease = 0.0f;
    float compressorAttack = 0.0f;
    float compressorRelease = 0.0f;

    void reset(float sampleRate) {
        gateEnvelope = compressorEnvelope = 0.0f;
        gateHold = 0;
        gateGain = 0.0f;
        holdFrames = static_cast<unsigned>(sampleRate * 0.1f);
        gateAttack = std::exp(-1.0f / (sampleRate * 0.002f));
        gateRelease = std::exp(-1.0f / (sampleRate * 0.15f));
        compressorAttack = std::exp(-1.0f / (sampleRate * 0.01f));
        compressorRelease = std::exp(-1.0f / (sampleRate * 0.15f));
    }

    void process(const float* input, float* output, unsigned frames, unsigned channels) {
        for (unsigned frame = 0; frame < frames; ++frame) {
            float level = 0.0f;
            for (unsigned channel = 0; channel < channels; ++channel)
                level = (level > std::fabs(input[frame * channels + channel]))
                    ? level : std::fabs(input[frame * channels + channel]);
            if (gate) {
                const float coefficient = level > gateEnvelope ? gateAttack : gateRelease;
                gateEnvelope = coefficient * gateEnvelope + (1.0f - coefficient) * level;
                if (gateEnvelope >= 0.005623f) gateHold = holdFrames; // -45 dB
                else if (gateHold) --gateHold;
                const float target = gateHold ? 1.0f : 0.0f;
                const float gainCoefficient = target > gateGain ? gateAttack : gateRelease;
                gateGain = gainCoefficient * gateGain + (1.0f - gainCoefficient) * target;
            }
            if (compressor) {
                const float coefficient = level > compressorEnvelope ? compressorAttack : compressorRelease;
                compressorEnvelope = coefficient * compressorEnvelope + (1.0f - coefficient) * level;
            }
            float gain = gate ? gateGain : 1.0f;
            if (compressor && compressorEnvelope > 0.125893f) // -18 dB, 2.5:1
                gain *= std::pow(0.125893f / compressorEnvelope, 0.6f);
            for (unsigned channel = 0; channel < channels; ++channel)
                output[frame * channels + channel] = input[frame * channels + channel] * gain;
        }
    }
};
