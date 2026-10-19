#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <vector>

enum class EffectModule : unsigned { HighPass, Noise, Gate, Expander, DeEsser, Eq, Compressor, Limiter };

// RBJ Audio EQ Cookbook biquads; coefficients are prepared outside APOProcess.
struct Biquad {
    double b0 = 1, b1 = 0, b2 = 0, a1 = 0, a2 = 0, z1 = 0, z2 = 0;

    float process(float input) {
        const double output = b0 * input + z1;
        z1 = b1 * input - a1 * output + z2;
        z2 = b2 * input - a2 * output;
        if (std::fabs(z1) < 1e-24) z1 = 0;
        if (std::fabs(z2) < 1e-24) z2 = 0;
        return static_cast<float>(output);
    }

    static Biquad make(float rate, float hz, float q, int type, float gain = 0) {
        const double frequency = (std::min)(static_cast<double>(hz), rate * 0.45);
        const double omega = 6.283185307179586 * frequency / rate;
        const double cosine = std::cos(omega), alpha = std::sin(omega) / (2 * q);
        const double amplitude = std::pow(10.0, gain / 40.0);
        Biquad filter;
        double a0;
        if (type == 0) {
            filter.b0 = (1 + cosine) / 2; filter.b1 = -(1 + cosine); filter.b2 = filter.b0;
            a0 = 1 + alpha; filter.a1 = -2 * cosine; filter.a2 = 1 - alpha;
        } else if (type == 1) {
            filter.b0 = alpha; filter.b1 = 0; filter.b2 = -alpha;
            a0 = 1 + alpha; filter.a1 = -2 * cosine; filter.a2 = 1 - alpha;
        } else {
            filter.b0 = 1 + alpha * amplitude; filter.b1 = -2 * cosine; filter.b2 = 1 - alpha * amplitude;
            a0 = 1 + alpha / amplitude; filter.a1 = -2 * cosine; filter.a2 = 1 - alpha / amplitude;
        }
        filter.b0 /= a0; filter.b1 /= a0; filter.b2 /= a0;
        filter.a1 /= a0; filter.a2 /= a0;
        return filter;
    }
};

struct Effects {
    std::array<EffectModule, 8> order = {{ EffectModule::HighPass, EffectModule::Noise,
        EffectModule::Gate, EffectModule::Expander, EffectModule::DeEsser, EffectModule::Eq,
        EffectModule::Compressor, EffectModule::Limiter }};
    bool highPass = false, gate = false, expander = false, deEsser = false, eq = false;
    bool compressor = false, limiter = false;
    int highPassHz = 80, highPassSlope = 12;
    int gateThresholdDb = -45;
    unsigned gateHoldMs = 100, gateReleaseMs = 150;
    int expanderThresholdDb = -40;
    float expanderRatio = 2;
    unsigned expanderAttackMs = 10, expanderReleaseMs = 50;
    int deEsserHz = 6000, deEsserThresholdDb = -30, deEsserMaxReductionDb = 6;
    float deEsserRatio = 4;
    unsigned deEsserAttackMs = 2, deEsserReleaseMs = 60;
    int eqLowHz = 150, eqMidHz = 3000, eqHighHz = 8000;
    int eqLowGainDb = 0, eqMidGainDb = 0, eqHighGainDb = 0;
    float eqLowQ = 1, eqMidQ = 1, eqHighQ = 1;
    int compressorThresholdDb = -18;
    float compressorRatio = 2.5f;
    unsigned compressorAttackMs = 10, compressorReleaseMs = 150;
    int limiterThresholdDb = -6;
    unsigned limiterReleaseMs = 60;

    struct Channel { Biquad highPass[2], equalizer[3], sibilance; };
    std::vector<Channel> channelState;
    float gateEnvelope = 0, gateGain = 0, compressorEnvelope = 0;
    float expanderPower = 0, expanderGain = 1, deEsserEnvelope = 0, limiterGain = 1;
    unsigned gateHold = 0, holdFrames = 0;
    float gateThreshold = 0, compressorThreshold = 0, compressorExponent = 0;
    float gateAttack = 0, gateRelease = 0, compressorAttack = 0, compressorRelease = 0;
    float expanderThreshold = 0, expanderAttack = 0, expanderRelease = 0, rmsDecay = 0;
    float deEsserThreshold = 0, deEsserMinimum = 0, deEsserAttack = 0, deEsserRelease = 0, deEsserExponent = 0;
    float limiterCeiling = 0, limiterRelease = 0;

    static float gain(float db) { return std::pow(10.0f, db / 20.0f); }
    static float decay(float rate, float milliseconds) { return std::exp(-1000.0f / (rate * milliseconds)); }

    void reset(float sampleRate, unsigned channels = 1) {
        gateEnvelope = gateGain = compressorEnvelope = expanderPower = deEsserEnvelope = 0;
        expanderGain = limiterGain = 1;
        gateHold = 0;
        holdFrames = static_cast<unsigned>(sampleRate * gateHoldMs / 1000.0f);
        gateThreshold = gain(static_cast<float>(gateThresholdDb));
        compressorThreshold = gain(static_cast<float>(compressorThresholdDb));
        compressorExponent = 1 - 1 / compressorRatio;
        gateAttack = decay(sampleRate, 2); gateRelease = decay(sampleRate, static_cast<float>(gateReleaseMs));
        compressorAttack = decay(sampleRate, static_cast<float>(compressorAttackMs));
        compressorRelease = decay(sampleRate, static_cast<float>(compressorReleaseMs));
        expanderThreshold = gain(static_cast<float>(expanderThresholdDb));
        expanderAttack = decay(sampleRate, static_cast<float>(expanderAttackMs));
        expanderRelease = decay(sampleRate, static_cast<float>(expanderReleaseMs));
        rmsDecay = decay(sampleRate, 10);
        deEsserThreshold = gain(static_cast<float>(deEsserThresholdDb));
        deEsserMinimum = gain(-static_cast<float>(deEsserMaxReductionDb));
        deEsserExponent = 1 - 1 / deEsserRatio;
        deEsserAttack = decay(sampleRate, static_cast<float>(deEsserAttackMs));
        deEsserRelease = decay(sampleRate, static_cast<float>(deEsserReleaseMs));
        limiterCeiling = gain(static_cast<float>(limiterThresholdDb));
        limiterRelease = decay(sampleRate, static_cast<float>(limiterReleaseMs));
        channelState.assign(channels, Channel{});
        for (auto& channel : channelState) {
            channel.highPass[0] = Biquad::make(sampleRate, static_cast<float>(highPassHz), highPassSlope == 24 ? 0.5411961f : 0.7071068f, 0);
            channel.highPass[1] = Biquad::make(sampleRate, static_cast<float>(highPassHz), 1.306563f, 0);
            channel.equalizer[0] = Biquad::make(sampleRate, static_cast<float>(eqLowHz), eqLowQ, 2, static_cast<float>(eqLowGainDb));
            channel.equalizer[1] = Biquad::make(sampleRate, static_cast<float>(eqMidHz), eqMidQ, 2, static_cast<float>(eqMidGainDb));
            channel.equalizer[2] = Biquad::make(sampleRate, static_cast<float>(eqHighHz), eqHighQ, 2, static_cast<float>(eqHighGainDb));
            channel.sibilance = Biquad::make(sampleRate, static_cast<float>(deEsserHz), 1, 1);
        }
    }

    bool enabled(EffectModule module) const {
        switch (module) {
        case EffectModule::HighPass: return highPass;
        case EffectModule::Gate: return gate;
        case EffectModule::Expander: return expander;
        case EffectModule::DeEsser: return deEsser;
        case EffectModule::Eq: return eq;
        case EffectModule::Compressor: return compressor;
        case EffectModule::Limiter: return limiter;
        default: return false;
        }
    }

    void processModule(EffectModule module, float* buffer, unsigned frames, unsigned channels) {
        if (!enabled(module) || channels != channelState.size()) return;
        for (unsigned frame = 0; frame < frames; ++frame) {
            float level = 0;
            for (unsigned channel = 0; channel < channels; ++channel) {
                float& sample = buffer[frame * channels + channel];
                auto& state = channelState[channel];
                if (module == EffectModule::HighPass) {
                    sample = state.highPass[0].process(sample);
                    if (highPassSlope == 24) sample = state.highPass[1].process(sample);
                } else if (module == EffectModule::Eq) {
                    for (auto& filter : state.equalizer) sample = filter.process(sample);
                }
                const float detected = module == EffectModule::DeEsser ? state.sibilance.process(sample) : sample;
                level = (std::max)(level, std::fabs(detected));
            }
            float multiplier = 1;
            if (module == EffectModule::Gate) {
                const float coefficient = level > gateEnvelope ? gateAttack : gateRelease;
                gateEnvelope = coefficient * gateEnvelope + (1 - coefficient) * level;
                const bool above = gateEnvelope >= gateThreshold;
                if (above) gateHold = holdFrames; else if (gateHold) --gateHold;
                const float target = above || gateHold ? 1.0f : 0.0f;
                const float smoothing = target > gateGain ? gateAttack : gateRelease;
                gateGain = smoothing * gateGain + (1 - smoothing) * target;
                multiplier = gateGain;
            } else if (module == EffectModule::Compressor) {
                const float coefficient = level > compressorEnvelope ? compressorAttack : compressorRelease;
                compressorEnvelope = coefficient * compressorEnvelope + (1 - coefficient) * level;
                if (compressorEnvelope > compressorThreshold)
                    multiplier = std::pow(compressorThreshold / compressorEnvelope, compressorExponent);
            } else if (module == EffectModule::Expander) {
                expanderPower = rmsDecay * expanderPower + (1 - rmsDecay) * level * level;
                const float rms = std::sqrt(expanderPower);
                const float target = rms < expanderThreshold ? std::pow((std::max)(rms, 1e-12f) / expanderThreshold, expanderRatio - 1) : 1;
                const float coefficient = target > expanderGain ? expanderAttack : expanderRelease;
                expanderGain = coefficient * expanderGain + (1 - coefficient) * (std::max)(target, 0.001f);
                multiplier = expanderGain;
            } else if (module == EffectModule::DeEsser) {
                const float coefficient = level > deEsserEnvelope ? deEsserAttack : deEsserRelease;
                deEsserEnvelope = coefficient * deEsserEnvelope + (1 - coefficient) * level;
                if (deEsserEnvelope > deEsserThreshold)
                    multiplier = (std::max)(deEsserMinimum, std::pow(deEsserThreshold / deEsserEnvelope, deEsserExponent));
            } else if (module == EffectModule::Limiter) {
                const float target = level > limiterCeiling ? limiterCeiling / level : 1;
                limiterGain = target < limiterGain ? target : limiterRelease * limiterGain + (1 - limiterRelease) * target;
                multiplier = limiterGain;
            }
            if (multiplier != 1) {
                for (unsigned channel = 0; channel < channels; ++channel)
                    buffer[frame * channels + channel] *= multiplier;
            }
        }
    }

    void process(const float* input, float* output, unsigned frames, unsigned channels) {
        if (input != output) std::copy(input, input + frames * channels, output);
        for (auto module : order) processModule(module, output, frames, channels);
    }
};
