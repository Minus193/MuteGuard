#include <windows.h>
#include <audioenginebaseapo.h>
#include <cassert>
#include <cmath>
#include "processing.h"

int main()
{
    HMODULE module = LoadLibraryW(L"x64\\Release\\MuteGuardAPO.dll");
    assert(module);
    auto getClass = reinterpret_cast<HRESULT(STDAPICALLTYPE*)(REFCLSID, REFIID, void**)>(
        GetProcAddress(module, "DllGetClassObject"));
    assert(getClass);
    const CLSID clsid = {0xA4EC941D, 0xF1BA, 0x4C19, {0xB1, 0x95, 0x40, 0x92, 0x85, 0x8D, 0x9D, 0x90}};
    IClassFactory* factory = nullptr;
    assert(SUCCEEDED(getClass(clsid, __uuidof(IClassFactory), reinterpret_cast<void**>(&factory))));
    IAudioProcessingObjectRT* apo = nullptr;
    assert(SUCCEEDED(factory->CreateInstance(nullptr, __uuidof(IAudioProcessingObjectRT), reinterpret_cast<void**>(&apo))));

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

    apo->Release();
    factory->Release();
    FreeLibrary(module);
}
