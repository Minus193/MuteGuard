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

    float samples[] = {1.0f, -0.5f, 0.0f};
    attenuate(samples, samples, 3);
    assert(std::fabs(samples[0] - 0.5f) < 0.0001f);
    assert(std::fabs(samples[1] + 0.25f) < 0.0001f);
    assert(samples[2] == 0.0f);

    apo->Release();
    factory->Release();
    FreeLibrary(module);
}
