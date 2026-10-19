#include <windows.h>
#include <audioenginebaseapo.h>
#include <cassert>
#include <cmath>
#include <string>
#include "processing.h"
#include "spectrum.h"
#include "preview.h"
#include <mmdeviceapi.h>
#include <cstdio>

static float toneLevel(Effects& effects, float frequency, float amplitude = 0.25f, float rate = 48000) {
    effects.reset(rate);
    const unsigned frames = static_cast<unsigned>(rate);
    std::vector<float> samples(frames);
    for (unsigned i = 0; i < frames; ++i)
        samples[i] = amplitude * std::sin(6.283185307f * frequency * i / rate);
    effects.process(samples.data(), samples.data(), frames, 1);
    double power = 0;
    for (unsigned i = frames / 2; i < frames; ++i) {
        assert(std::isfinite(samples[i]));
        power += samples[i] * samples[i];
    }
    return static_cast<float>(std::sqrt(power / (frames - frames / 2)));
}

static void testRack() {
    Effects filter;
    filter.highPass = true;
    const float bass = toneLevel(filter, 20);
    const float voice = toneLevel(filter, 1000);
    assert(bass < voice * 0.07f && voice > 0.17f);
    filter.highPassSlope = 24;
    assert(toneLevel(filter, 20) < bass * 0.1f);

    Effects expansion;
    expansion.expander = true;
    assert(toneLevel(expansion, 1000, 0.001f) < 0.0001f);
    expansion.expanderRatio = 1;
    assert(toneLevel(expansion, 1000, 0.001f) > 0.0007f);

    Effects sibilance;
    sibilance.deEsser = true;
    assert(toneLevel(sibilance, 6000) < toneLevel(sibilance, 500) * 0.65f);
    sibilance.deEsserRatio = 1;
    assert(toneLevel(sibilance, 6000) > 0.17f);

    Effects equalizer;
    equalizer.eq = true;
    const float flat = toneLevel(equalizer, 3000);
    equalizer.eqMidGainDb = 6;
    assert(toneLevel(equalizer, 3000) > flat * 1.98f);
    equalizer.eqMidGainDb = -6;
    assert(toneLevel(equalizer, 3000) < flat * 0.51f);
    equalizer.eqMidGainDb = 6;
    equalizer.eqMidQ = 0.3f;
    const float broad = toneLevel(equalizer, 4500);
    equalizer.eqMidQ = 10;
    assert(toneLevel(equalizer, 4500) < broad * 0.75f);

    Effects ceiling;
    ceiling.limiter = true;
    ceiling.eq = true;
    ceiling.eqMidGainDb = 12;
    ceiling.reset(48000, 2);
    std::vector<float> stereo(9600);
    for (unsigned i = 0; i < 4800; ++i) stereo[2 * i] = std::sin(6.283185307f * 3000 * i / 48000);
    ceiling.process(stereo.data(), stereo.data(), 4800, 2);
    for (unsigned i = 0; i < 4800; ++i) {
        assert(std::fabs(stereo[2 * i]) <= Effects::gain(-6) + 1e-6f);
        assert(stereo[2 * i + 1] == 0);
    }
    const float limited = toneLevel(ceiling, 3000, 0.8f);
    std::swap(ceiling.order[5], ceiling.order[7]);
    assert(toneLevel(ceiling, 3000, 0.8f) > limited * 2);

    for (float rate : { 16000.0f, 44100.0f, 48000.0f, 96000.0f }) {
        Effects all;
        all.highPass = all.expander = all.deEsser = all.eq = all.compressor = all.limiter = true;
        assert(std::isfinite(toneLevel(all, 1000, 0.25f, rate)));
    }
}

static float graphLevel(IAudioProcessingObjectRT* apo) {
    IAudioProcessingObjectConfiguration* configuration = nullptr;
    assert(SUCCEEDED(apo->QueryInterface(__uuidof(IAudioProcessingObjectConfiguration), reinterpret_cast<void**>(&configuration))));
    WAVEFORMATEX wave = {};
    wave.wFormatTag = WAVE_FORMAT_IEEE_FLOAT;
    wave.nChannels = 1; wave.nSamplesPerSec = 48000; wave.wBitsPerSample = 32;
    wave.nBlockAlign = 4; wave.nAvgBytesPerSec = 192000;
    IAudioMediaType* format = nullptr;
    assert(SUCCEEDED(CreateAudioMediaType(&wave, sizeof(wave), &format)));
    std::vector<float> samples(4800), output(4800);
    for (unsigned i = 0; i < samples.size(); ++i) samples[i] = 0.8f * std::sin(6.283185307f * 3000 * i / 48000);
    APO_CONNECTION_DESCRIPTOR inputDescriptor = { APO_CONNECTION_BUFFER_TYPE_EXTERNAL,
        reinterpret_cast<UINT_PTR>(samples.data()), 4800, format, APO_CONNECTION_DESCRIPTOR_SIGNATURE };
    APO_CONNECTION_DESCRIPTOR outputDescriptor = { APO_CONNECTION_BUFFER_TYPE_EXTERNAL,
        reinterpret_cast<UINT_PTR>(output.data()), 4800, format, APO_CONNECTION_DESCRIPTOR_SIGNATURE };
    auto* inputDescriptorPointer = &inputDescriptor;
    auto* outputDescriptorPointer = &outputDescriptor;
    assert(SUCCEEDED(configuration->LockForProcess(1, &inputDescriptorPointer, 1, &outputDescriptorPointer)));
    APO_CONNECTION_PROPERTY input = {}, result = {};
    input.pBuffer = reinterpret_cast<UINT_PTR>(samples.data()); input.u32ValidFrameCount = 4800;
    input.u32BufferFlags = BUFFER_VALID; input.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;
    result.pBuffer = reinterpret_cast<UINT_PTR>(output.data()); result.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;
    auto* inputPointer = &input; auto* resultPointer = &result;
    apo->APOProcess(1, &inputPointer, 1, &resultPointer);
    assert(result.u32ValidFrameCount == 4800 && result.u32BufferFlags == BUFFER_VALID);
    float peak = 0;
    for (float sample : output) { assert(std::isfinite(sample)); peak = (std::max)(peak, std::fabs(sample)); }
    assert(SUCCEEDED(configuration->UnlockForProcess()));
    configuration->Release(); format->Release();
    return peak;
}

static void testConfiguredOrder(IAudioProcessingObjectRT* apo) {
    wchar_t previous[MAX_PATH] = {}, temporary[MAX_PATH] = {};
    GetEnvironmentVariableW(L"ProgramData", previous, MAX_PATH);
    assert(GetTempPathW(MAX_PATH, temporary) > 0);
    const std::wstring root = std::wstring(temporary) + L"MuteGuardRackTest-" + std::to_wstring(GetCurrentProcessId());
    assert(CreateDirectoryW(root.c_str(), nullptr));
    const auto application = root + L"\\MuteGuard";
    const auto state = application + L"\\APO-test";
    assert(CreateDirectoryW(application.c_str(), nullptr));
    assert(CreateDirectoryW(state.c_str(), nullptr));
    const auto ini = state + L"\\effects.ini";
    assert(SetEnvironmentVariableW(L"ProgramData", root.c_str()));
    assert(WritePrivateProfileStringW(L"Effects", L"Eq", L"1", ini.c_str()));
    assert(WritePrivateProfileStringW(L"Effects", L"EqMidGainDb", L"12", ini.c_str()));
    assert(WritePrivateProfileStringW(L"Effects", L"Limiter", L"1", ini.c_str()));
    assert(WritePrivateProfileStringW(L"Effects", L"LimiterThresholdDb", L"-6", ini.c_str()));
    const float finalLimiter = graphLevel(apo);
    assert(finalLimiter <= Effects::gain(-6) + 1e-6f);
    assert(WritePrivateProfileStringW(L"Effects", L"Order5", L"7", ini.c_str()));
    assert(WritePrivateProfileStringW(L"Effects", L"Order7", L"5", ini.c_str()));
    assert(graphLevel(apo) > finalLimiter * 2);
    assert(WritePrivateProfileStringW(L"Effects", L"Order0", L"7", ini.c_str()));
    assert(graphLevel(apo) <= Effects::gain(-6) + 1e-6f);
    assert(SetEnvironmentVariableW(L"ProgramData", previous[0] ? previous : nullptr));
    assert(DeleteFileW(ini.c_str()));
    assert(RemoveDirectoryW(state.c_str()));
    assert(RemoveDirectoryW(application.c_str()));
    assert(RemoveDirectoryW(root.c_str()));
}

static void testSpectrum() {
    Spectrum spectrum;
    float bins[64];
    spectrum.read(48000, bins);
    for (float value : bins) assert(value == -100);
    for (unsigned i = 0; i < Spectrum::size; ++i)
        spectrum.push(0.5f * std::sin(6.283185307f * 1500 * i / 48000));
    spectrum.read(48000, bins);
    const unsigned band = unsigned(std::max_element(bins, bins + 64) - bins);
    assert(band == unsigned(std::log(1500.0 / 20) / std::log(1000.0) * 64));
    assert(std::fabs(bins[band] + 6.0206f) < 0.05f);
    spectrum.read(16000, bins);
    for (unsigned i = 56; i < 64; ++i) assert(bins[i] == -100);
}

static void testPreview(HMODULE module, bool microphone) {
    auto start = reinterpret_cast<void*(__cdecl*)(const wchar_t*)>(GetProcAddress(module, "MuteGuardPreviewStart"));
    auto read = reinterpret_cast<int(__cdecl*)(void*, PreviewSnapshot*, unsigned)>(GetProcAddress(module, "MuteGuardPreviewRead"));
    auto update = reinterpret_cast<void(__cdecl*)(void*)>(GetProcAddress(module, "MuteGuardPreviewUpdate"));
    auto stop = reinterpret_cast<void(__cdecl*)(void*)>(GetProcAddress(module, "MuteGuardPreviewStop"));
    assert(start && read && update && stop);
    assert(!start(nullptr) && !start(L""));
    PreviewSnapshot snapshot;
    void* preview = start(L"invalid-device-for-test");
    assert(preview);
    assert(!read(preview, &snapshot, sizeof(snapshot) - 1));
    for (unsigned i = 0; i < 100; ++i) {
        assert(read(preview, &snapshot, sizeof(snapshot)));
        if (snapshot.status < 0) break;
        Sleep(20);
    }
    assert(snapshot.status < 0 && FAILED(snapshot.error));
    stop(preview); stop(nullptr);
    if (!microphone) return;
    assert(SUCCEEDED(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED)));
    IMMDeviceEnumerator* enumerator = nullptr;
    assert(SUCCEEDED(CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL, IID_PPV_ARGS(&enumerator))));
    IMMDevice* device = nullptr;
    assert(SUCCEEDED(enumerator->GetDefaultAudioEndpoint(eCapture, eCommunications, &device)));
    LPWSTR id = nullptr;
    assert(SUCCEEDED(device->GetId(&id)));
    preview = start(id); assert(preview);
    CoTaskMemFree(id); device->Release(); enumerator->Release();
    for (unsigned i = 0; i < 200; ++i) {
        assert(read(preview, &snapshot, sizeof(snapshot)));
        if (snapshot.status < 0 || snapshot.sequence > 5) break;
        Sleep(20);
    }
    printf("Preview status=%d error=0x%08X rate=%u channels=%u sequence=%u peak=%.1f / %.1f\n",
        snapshot.status, unsigned(snapshot.error), snapshot.rate, snapshot.channels, snapshot.sequence, snapshot.inputPeak, snapshot.outputPeak);
    assert(snapshot.status == 1 && snapshot.sequence > 5);
    for (float value : snapshot.input) assert(std::isfinite(value) && value >= -100 && value <= 0);
    update(preview);
    for (unsigned i = 0; i < 100; ++i) {
        Sleep(20); assert(read(preview, &snapshot, sizeof(snapshot)));
        if (snapshot.revision == 1) break;
    }
    assert(snapshot.status == 1 && snapshot.revision == 1);
    stop(preview); CoUninitialize();
}

int main(int argc, char** argv)
{
    HMODULE module = LoadLibraryW(L"x64\\Release\\MuteGuardAPO.dll");
    assert(module);
    testSpectrum();
    testPreview(module, argc > 1 && std::string(argv[1]) == "--microphone");
    auto getClass = reinterpret_cast<HRESULT(STDAPICALLTYPE*)(REFCLSID, REFIID, void**)>(
        GetProcAddress(module, "DllGetClassObject"));
    assert(getClass);
    const CLSID clsid = {0xA4EC941D, 0xF1BA, 0x4C19, {0xB1, 0x95, 0x40, 0x92, 0x85, 0x8D, 0x9D, 0x90}};
    IClassFactory* factory = nullptr;
    assert(SUCCEEDED(getClass(clsid, __uuidof(IClassFactory), reinterpret_cast<void**>(&factory))));
    IAudioProcessingObjectRT* apo = nullptr;
    assert(SUCCEEDED(factory->CreateInstance(nullptr, __uuidof(IAudioProcessingObjectRT), reinterpret_cast<void**>(&apo))));
    IAudioProcessingObject* control = nullptr;
    assert(SUCCEEDED(apo->QueryInterface(__uuidof(IAudioProcessingObject), reinterpret_cast<void**>(&control))));
    APOInitSystemEffects init = {};
    init.APOInit.cbSize = sizeof(init);
    init.APOInit.clsid = clsid;
    assert(SUCCEEDED(control->Initialize(sizeof(init), reinterpret_cast<BYTE*>(&init))));
    assert(control->Initialize(sizeof(init), reinterpret_cast<BYTE*>(&init)) == APOERR_ALREADY_INITIALIZED);
    control->Release();

    Effects effects;
    effects.reset(48000.0f);
    float samples[] = {1.0f, -0.5f, 0.0f};
    effects.process(samples, samples, 3, 1);
    assert(samples[0] == 1.0f && samples[1] == -0.5f && samples[2] == 0.0f);

    effects.gate = true;
    effects.reset(48000.0f);
    float quiet[4800];
    for (float& sample : quiet) sample = 0.001f;
    effects.process(quiet, quiet, 4800, 1);
    assert(std::fabs(quiet[4799]) < 0.0001f);

    effects.gate = false;
    effects.compressor = true;
    effects.reset(48000.0f);
    float loud[4800];
    for (float& sample : loud) sample = 1.0f;
    effects.process(loud, loud, 4800, 1);
    assert(loud[4799] < 0.4f && loud[4799] > 0.2f);

    effects.compressorRatio = 1.0f;
    effects.reset(48000.0f);
    for (float& sample : loud) sample = 1.0f;
    effects.process(loud, loud, 4800, 1);
    assert(loud[4799] == 1.0f);

    effects.compressor = false;
    effects.gate = true;
    effects.gateThresholdDb = -70;
    effects.reset(48000.0f);
    for (float& sample : quiet) sample = 0.001f;
    effects.process(quiet, quiet, 4800, 1);
    assert(quiet[4799] > 0.0009f);

    effects.gateHoldMs = 0;
    effects.reset(48000.0f);
    for (float& sample : loud) sample = 1.0f;
    effects.process(loud, loud, 4800, 1);
    assert(loud[4799] > 0.99f);

    testRack();
    testConfiguredOrder(apo);

    apo->Release();
    factory->Release();
    FreeLibrary(module);
}
