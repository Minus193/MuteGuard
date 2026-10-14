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
    int gateThresholdDb = -45;
    unsigned gateHoldMs = 100;
    unsigned gateReleaseMs = 150;
    int compressorThresholdDb = -18;
    float compressorRatio = 2.5f;
    unsigned compressorAttackMs = 10;
    unsigned compressorReleaseMs = 150;
    float gateThreshold = 0.005623f;
    float compressorThreshold = 0.125893f;
    float compressorExponent = 0.6f;
    float gateAttack = 0.0f;
    float gateRelease = 0.0f;
    float compressorAttack = 0.0f;
    float compressorRelease = 0.0f;

    void reset(float sampleRate) {
        gateEnvelope = compressorEnvelope = 0.0f;
        gateHold = 0;
        gateGain = 0.0f;
        holdFrames = static_cast<unsigned>(sampleRate * gateHoldMs / 1000.0f);
        gateThreshold = std::pow(10.0f, gateThresholdDb / 20.0f);
        compressorThreshold = std::pow(10.0f, compressorThresholdDb / 20.0f);
        compressorExponent = 1.0f - 1.0f / compressorRatio;
        gateAttack = std::exp(-1.0f / (sampleRate * 0.002f));
        gateRelease = std::exp(-1000.0f / (sampleRate * gateReleaseMs));
        compressorAttack = std::exp(-1000.0f / (sampleRate * compressorAttackMs));
        compressorRelease = std::exp(-1000.0f / (sampleRate * compressorReleaseMs));
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
                const bool aboveThreshold = gateEnvelope >= gateThreshold;
                if (aboveThreshold) gateHold = holdFrames;
                else if (gateHold) --gateHold;
                const float target = aboveThreshold || gateHold ? 1.0f : 0.0f;
                const float gainCoefficient = target > gateGain ? gateAttack : gateRelease;
                gateGain = gainCoefficient * gateGain + (1.0f - gainCoefficient) * target;
            }
            if (compressor) {
                const float coefficient = level > compressorEnvelope ? compressorAttack : compressorRelease;
                compressorEnvelope = coefficient * compressorEnvelope + (1.0f - coefficient) * level;
            }
            float gain = gate ? gateGain : 1.0f;
            if (compressor && compressorEnvelope > compressorThreshold)
                gain *= std::pow(compressorThreshold / compressorEnvelope, compressorExponent);
            for (unsigned channel = 0; channel < channels; ++channel)
                output[frame * channels + channel] = input[frame * channels + channel] * gain;
        }
    }
};
