#pragma once
#include <array>
#include <algorithm>
#include <cmath>
#include <complex>

struct Spectrum {
    static constexpr unsigned size = 4096, bands = 64;
    std::array<float, size> samples{};
    unsigned position = 0;
    static float db(float amplitude) {
        return 20 * std::log10((std::max)(amplitude, 0.00001f));
    }
    void push(float sample) { samples[position++ % size] = sample; }
    void read(unsigned rate, float* output) const {
        std::array<std::complex<float>, size> fft;
        constexpr float pi = 3.14159265358979323846f;
        for (unsigned i = 0; i < size; ++i)
            fft[i] = samples[(position + i) % size] * (0.5f - 0.5f * std::cos(2 * pi * i / size));
        for (unsigned i = 1, j = 0; i < size; ++i) {
            unsigned bit = size >> 1;
            for (; j & bit; bit >>= 1) j ^= bit;
            j ^= bit;
            if (i < j) std::swap(fft[i], fft[j]);
        }
        for (unsigned length = 2; length <= size; length *= 2) {
            const auto step = std::polar(1.0f, -2 * pi / length);
            for (unsigned start = 0; start < size; start += length) {
                std::complex<float> weight(1, 0);
                for (unsigned j = 0; j < length / 2; ++j) {
                    const auto even = fft[start + j], odd = weight * fft[start + j + length / 2];
                    fft[start + j] = even + odd;
                    fft[start + j + length / 2] = even - odd;
                    weight *= step;
                }
            }
        }
        for (unsigned band = 0; band < bands; ++band) {
            const float low = 20 * std::pow(1000.0f, band / float(bands));
            const float high = 20 * std::pow(1000.0f, (band + 1) / float(bands));
            if (low >= rate / 2.0f) { output[band] = -100; continue; }
            const unsigned first = (std::max)(1u, unsigned(std::ceil(low * size / rate)));
            const unsigned last = (std::min)(size / 2, (std::max)(first, unsigned(std::floor(high * size / rate))));
            float peak = 0;
            for (unsigned bin = first; bin <= last; ++bin) peak = (std::max)(peak, std::abs(fft[bin]) * 4 / size);
            output[band] = (std::min)(0.0f, db(peak));
        }
    }
};
