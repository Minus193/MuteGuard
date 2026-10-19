#include "stdafx.h"
#include "MuteGuardAPO.h"
#include "spectrum.h"
#include "preview.h"
#include "monitor-buffer.h"
#include <audioclient.h>
#include <mmdeviceapi.h>
#include <wrl/client.h>
#include <atomic>
#include <cstdint>
#include <cstring>
#include <memory>
#include <mutex>
#include <thread>

using Microsoft::WRL::ComPtr;

struct Preview {
    HANDLE stop = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    HANDLE control = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    std::atomic<int> monitorVolume{ -1 };
    std::atomic<unsigned> revision{ 0 };
    std::mutex mutex;
    PreviewSnapshot snapshot;
    std::thread worker;
    ~Preview() {
        if (stop) SetEvent(stop);
        if (worker.joinable()) worker.join();
        if (stop) CloseHandle(stop);
        if (control) CloseHandle(control);
    }
    void run(const std::wstring& id) noexcept;
    HRESULT capture(const std::wstring& id);
};

struct EventHandle {
    HANDLE value = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    ~EventHandle() { if (value) CloseHandle(value); }
};

// All COM objects and microphone buffers stay on this worker thread.
void Preview::run(const std::wstring& id) noexcept {
    const HRESULT initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    HRESULT result = initialized;
    if (SUCCEEDED(initialized)) {
        try { result = capture(id); }
        catch (const std::bad_alloc&) { result = E_OUTOFMEMORY; }
        catch (...) { result = E_FAIL; }
        CoUninitialize();
    }
    if (FAILED(result)) {
        std::lock_guard<std::mutex> lock(mutex);
        snapshot.status = -1; snapshot.error = result;
    }
}

#define CHECK_PREVIEW(call) do { const HRESULT hr = (call); if (FAILED(hr)) return hr; } while (0)

struct Monitor {
    ComPtr<IAudioClient> client;
    ComPtr<IAudioRenderClient> render;
    ComPtr<ISimpleAudioVolume> volume;
    EventHandle event;
    MonitorBuffer queue;
    UINT32 capacity = 0;
    explicit Monitor(unsigned rate) : queue(rate / 10) {}
    ~Monitor() { if (client) client->Stop(); }
    HRESULT open(IMMDeviceEnumerator* enumerator, unsigned rate, int percent) {
        if (!event.value) return HRESULT_FROM_WIN32(GetLastError());
        ComPtr<IMMDevice> device;
        CHECK_PREVIEW(enumerator->GetDefaultAudioEndpoint(eRender, eMultimedia, &device));
        CHECK_PREVIEW(device->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, &client));
        WAVEFORMATEX wave = {};
        wave.wFormatTag = WAVE_FORMAT_IEEE_FLOAT; wave.nChannels = 2; wave.nSamplesPerSec = rate;
        wave.wBitsPerSample = 32; wave.nBlockAlign = 8; wave.nAvgBytesPerSec = rate * 8;
        GUID session;
        CHECK_PREVIEW(CoCreateGuid(&session));
        CHECK_PREVIEW(client->Initialize(AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM |
            AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY | AUDCLNT_STREAMFLAGS_NOPERSIST,
            0, 0, &wave, &session));
        CHECK_PREVIEW(client->SetEventHandle(event.value));
        CHECK_PREVIEW(client->GetBufferSize(&capacity));
        CHECK_PREVIEW(client->GetService(IID_PPV_ARGS(&render)));
        CHECK_PREVIEW(client->GetService(IID_PPV_ARGS(&volume)));
        CHECK_PREVIEW(volume->SetMasterVolume(percent / 100.0f, nullptr));
        return client->Start();
    }
    HRESULT write() {
        UINT32 padding = 0;
        CHECK_PREVIEW(client->GetCurrentPadding(&padding));
        if (padding > capacity) return E_UNEXPECTED;
        const UINT32 available = capacity - padding;
        if (!available) return S_OK;
        BYTE* buffer = nullptr;
        CHECK_PREVIEW(render->GetBuffer(available, &buffer));
        queue.pop(reinterpret_cast<float*>(buffer), available);
        return render->ReleaseBuffer(available, 0);
    }
};

HRESULT Preview::capture(const std::wstring& id) {
    ComPtr<IMMDeviceEnumerator> enumerator;
    CHECK_PREVIEW(CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL, IID_PPV_ARGS(&enumerator)));
    ComPtr<IMMDevice> device;
    CHECK_PREVIEW(enumerator->GetDevice(id.c_str(), &device));
    ComPtr<IAudioClient2> client;
    CHECK_PREVIEW(device->Activate(__uuidof(IAudioClient2), CLSCTX_ALL, nullptr, &client));
    AudioClientProperties properties = {};
    properties.cbSize = sizeof(properties);
    properties.eCategory = AudioCategory_Other;
    properties.Options = AUDCLNT_STREAMOPTIONS_RAW;
    CHECK_PREVIEW(client->SetClientProperties(&properties));
    WAVEFORMATEX* allocated = nullptr;
    CHECK_PREVIEW(client->GetMixFormat(&allocated));
    std::unique_ptr<WAVEFORMATEX, decltype(&CoTaskMemFree)> format(allocated, CoTaskMemFree);
    const unsigned channels = format->nChannels, rate = format->nSamplesPerSec;
    const unsigned bytes = format->wBitsPerSample / 8;
    bool floating = format->wFormatTag == WAVE_FORMAT_IEEE_FLOAT;
    bool pcm = format->wFormatTag == WAVE_FORMAT_PCM;
    if (format->wFormatTag == WAVE_FORMAT_EXTENSIBLE && format->cbSize >= 22) {
        const auto* extended = reinterpret_cast<WAVEFORMATEXTENSIBLE*>(format.get());
        const GUID floatType = { 3, 0, 0x0010, { 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71 } };
        const GUID pcmType = { 1, 0, 0x0010, { 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71 } };
        floating = IsEqualGUID(extended->SubFormat, floatType);
        pcm = IsEqualGUID(extended->SubFormat, pcmType);
    }
    if (!channels || channels > 32 || rate < 8000 || rate > 384000 || format->nBlockAlign != channels * bytes ||
        !((floating && bytes == 4) || (pcm && bytes >= 2 && bytes <= 4))) return AUDCLNT_E_UNSUPPORTED_FORMAT;
    EventHandle audioEvent;
    if (!audioEvent.value) return HRESULT_FROM_WIN32(GetLastError());
    CHECK_PREVIEW(client->Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
        1000000, 0, format.get(), nullptr));
    CHECK_PREVIEW(client->SetEventHandle(audioEvent.value));
    ComPtr<IAudioCaptureClient> capture;
    CHECK_PREVIEW(client->GetService(IID_PPV_ARGS(&capture)));
    UINT32 capacity = 0;
    CHECK_PREVIEW(client->GetBufferSize(&capacity));
    if (!capacity || capacity > rate * 2) return E_UNEXPECTED;
    std::vector<float> input(size_t(capacity) * channels), output(input.size());
    auto apo = std::make_unique<MuteGuardAPO>(nullptr);
    APOInitSystemEffects init = {};
    init.APOInit.cbSize = sizeof(init); init.APOInit.clsid = __uuidof(MuteGuardAPO);
    CHECK_PREVIEW(apo->Initialize(sizeof(init), reinterpret_cast<BYTE*>(&init)));
    WAVEFORMATEX wave = {};
    wave.wFormatTag = WAVE_FORMAT_IEEE_FLOAT; wave.nChannels = static_cast<WORD>(channels);
    wave.nSamplesPerSec = rate; wave.wBitsPerSample = 32;
    wave.nBlockAlign = static_cast<WORD>(channels * 4); wave.nAvgBytesPerSec = rate * wave.nBlockAlign;
    ComPtr<IAudioMediaType> media;
    CHECK_PREVIEW(CreateAudioMediaType(&wave, sizeof(wave), &media));
    APO_CONNECTION_DESCRIPTOR inDesc = { APO_CONNECTION_BUFFER_TYPE_EXTERNAL,
        reinterpret_cast<UINT_PTR>(input.data()), capacity, media.Get(), APO_CONNECTION_DESCRIPTOR_SIGNATURE };
    APO_CONNECTION_DESCRIPTOR outDesc = { APO_CONNECTION_BUFFER_TYPE_EXTERNAL,
        reinterpret_cast<UINT_PTR>(output.data()), capacity, media.Get(), APO_CONNECTION_DESCRIPTOR_SIGNATURE };
    auto* inDescPtr = &inDesc; auto* outDescPtr = &outDesc;
    CHECK_PREVIEW(apo->LockForProcess(1, &inDescPtr, 1, &outDescPtr));
    struct Unlock { MuteGuardAPO* apo; ~Unlock() { apo->UnlockForProcess(); } } unlock{ apo.get() };
    CHECK_PREVIEW(client->Start());
    struct Stop { IAudioClient* client; ~Stop() { client->Stop(); } } stopClient{ client.Get() };
    Spectrum before, after;
    unsigned applied = revision.load(), accumulated = 0;
    float inputPeak = 0, outputPeak = 0;
    std::unique_ptr<Monitor> monitor;
    PreviewSnapshot next;
    next.status = 1; next.rate = rate; next.channels = channels;
    ULONGLONG lastCapture = GetTickCount64();
    const auto publishMonitor = [&] (HRESULT error) {
        if (FAILED(error)) { monitor.reset(); monitorVolume.store(-1); }
        next.monitorStatus = FAILED(error) ? -1 : (monitor ? 1 : 0);
        next.monitorError = error;
        std::lock_guard<std::mutex> lock(mutex); snapshot = next;
    };
    for (;;) {
        HANDLE events[] = { stop, control, audioEvent.value, monitor ? monitor->event.value : nullptr };
        const DWORD ready = WaitForMultipleObjects(monitor ? 4 : 3, events, FALSE, 2000);
        if (ready == WAIT_OBJECT_0) return S_OK;
        if (GetTickCount64() - lastCapture > 2000) return HRESULT_FROM_WIN32(ERROR_TIMEOUT);
        if (ready == WAIT_OBJECT_0 + 1) {
            const int percent = monitorVolume.load();
            HRESULT result = S_OK;
            if (percent < 0) monitor.reset();
            else if (!monitor) {
                monitor = std::make_unique<Monitor>(rate);
                result = monitor->open(enumerator.Get(), rate, percent);
            } else result = monitor->volume->SetMasterVolume(percent / 100.0f, nullptr);
            publishMonitor(result);
            continue;
        }
        if (ready == WAIT_OBJECT_0 + 3 && monitor) {
            const HRESULT result = monitor->write();
            if (FAILED(result)) publishMonitor(result);
            continue;
        }
        if (ready != WAIT_OBJECT_0 + 2) return ready == WAIT_TIMEOUT ? HRESULT_FROM_WIN32(ERROR_TIMEOUT) : HRESULT_FROM_WIN32(GetLastError());
        lastCapture = GetTickCount64();
        const unsigned requested = revision.load();
        if (applied != requested) {
            CHECK_PREVIEW(apo->UnlockForProcess());
            CHECK_PREVIEW(apo->LockForProcess(1, &inDescPtr, 1, &outDescPtr));
            applied = requested; accumulated = 0; inputPeak = outputPeak = 0;
            before = Spectrum{}; after = Spectrum{};
            if (monitor) monitor->queue.head = monitor->queue.count = 0;
        }
        UINT32 packet = 0;
        CHECK_PREVIEW(capture->GetNextPacketSize(&packet));
        while (packet) {
            BYTE* data = nullptr; UINT32 frames = 0; DWORD flags = 0;
            CHECK_PREVIEW(capture->GetBuffer(&data, &frames, &flags, nullptr, nullptr));
            if (frames > capacity) { capture->ReleaseBuffer(frames); return E_UNEXPECTED; }
            for (size_t i = 0; i < size_t(frames) * channels; ++i) {
                float value = 0;
                if (!(flags & AUDCLNT_BUFFERFLAGS_SILENT)) {
                    const BYTE* sample = data + i * bytes;
                    if (floating) std::memcpy(&value, sample, 4);
                    else {
                        // Signed little-endian PCM, including 24-bit containers.
                        int32_t signedValue = int32_t(int8_t(sample[bytes - 1]));
                        for (int byte = int(bytes) - 2; byte >= 0; --byte) signedValue = signedValue * 256 + sample[byte];
                        value = float(signedValue / std::pow(2.0, bytes * 8 - 1));
                    }
                }
                input[i] = std::isfinite(value) ? value : 0;
            }
            CHECK_PREVIEW(capture->ReleaseBuffer(frames));
            if (flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY) {
                before = Spectrum{}; after = Spectrum{};
                if (monitor) monitor->queue.head = monitor->queue.count = 0;
            }
            APO_CONNECTION_PROPERTY in = {}, out = {};
            in.pBuffer = reinterpret_cast<UINT_PTR>(input.data()); in.u32ValidFrameCount = frames;
            in.u32BufferFlags = BUFFER_VALID; in.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;
            out.pBuffer = reinterpret_cast<UINT_PTR>(output.data()); out.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;
            auto* inPtr = &in; auto* outPtr = &out;
            apo->APOProcess(1, &inPtr, 1, &outPtr);
            if (monitor) monitor->queue.push(output.data(), frames, channels);
            for (unsigned frame = 0; frame < frames; ++frame) {
                // Show channel 1's spectrum; meters include every channel without phase cancellation.
                before.push(input[frame * channels]); after.push(output[frame * channels]);
                for (unsigned channel = 0; channel < channels; ++channel) {
                    inputPeak = (std::max)(inputPeak, std::fabs(input[frame * channels + channel]));
                    outputPeak = (std::max)(outputPeak, std::fabs(output[frame * channels + channel]));
                }
            }
            accumulated += frames;
            if (accumulated >= rate / 15) {
                before.read(rate, next.input); after.read(rate, next.output);
                next.inputPeak = Spectrum::db(inputPeak); next.outputPeak = Spectrum::db(outputPeak);
                HNSTIME latency = 0; apo->GetLatency(&latency); next.noiseActive = latency != 0;
                next.revision = applied; ++next.sequence;
                { std::lock_guard<std::mutex> lock(mutex); snapshot = next; }
                accumulated = 0; inputPeak = outputPeak = 0;
            }
            CHECK_PREVIEW(capture->GetNextPacketSize(&packet));
        }
    }
}

extern "C" void* __cdecl MuteGuardPreviewStart(const wchar_t* deviceId) noexcept {
    if (!deviceId || !*deviceId) return nullptr;
    try {
        auto preview = std::make_unique<Preview>();
        if (!preview->stop || !preview->control) return nullptr;
        const std::wstring id(deviceId);
        preview->worker = std::thread([state = preview.get(), id] { state->run(id); });
        return preview.release();
    } catch (...) { return nullptr; }
}
extern "C" int __cdecl MuteGuardPreviewRead(Preview* preview, PreviewSnapshot* result, unsigned size) noexcept {
    if (!preview || !result || size != sizeof(PreviewSnapshot)) return 0;
    std::lock_guard<std::mutex> lock(preview->mutex); *result = preview->snapshot; return 1;
}
extern "C" void __cdecl MuteGuardPreviewUpdate(Preview* preview) noexcept { if (preview) ++preview->revision; }
extern "C" void __cdecl MuteGuardPreviewStop(Preview* preview) noexcept { delete preview; }
extern "C" void __cdecl MuteGuardPreviewListen(Preview* preview, int percent) noexcept {
    if (!preview) return;
    preview->monitorVolume.store((std::max)(-1, (std::min)(100, percent)));
    SetEvent(preview->control);
}
