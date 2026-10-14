#include "stdafx.h"

#include "MuteGuardAPO.h"
#include "processing.h"
#include <cstdio>

using namespace std;

static void TraceApo(const char* stage, HRESULT result, LONG buffers = 0)
{
	wchar_t path[MAX_PATH] = {};
	const DWORD length = GetEnvironmentVariableW(L"ProgramData", path, MAX_PATH);
	if (!length || length >= MAX_PATH || wcscat_s(path, L"\\MuteGuard\\APO-test\\trace.log") != 0)
		return;
	HANDLE file = CreateFileW(path, FILE_APPEND_DATA, FILE_SHARE_READ | FILE_SHARE_WRITE,
		NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
	if (file == INVALID_HANDLE_VALUE)
		return;
	char line[128] = {};
	const int size = sprintf_s(line, "%s result=0x%08X buffers=%ld\r\n", stage, static_cast<unsigned>(result), buffers);
	DWORD written = 0;
	if (size > 0)
		WriteFile(file, line, static_cast<DWORD>(size), &written, NULL);
	CloseHandle(file);
}

static int ReadBounded(const wchar_t* key, int fallback, int low, int high, const wchar_t* path)
{
	const int value = static_cast<int>(GetPrivateProfileIntW(L"Effects", key, fallback, path));
	return value < low ? low : (value > high ? high : value);
}

long MuteGuardAPO::instCount = 0;
const CRegAPOProperties<1> MuteGuardAPO::regProperties(
	__uuidof(MuteGuardAPO), L"MuteGuard APO Prototype", L"MuteGuard", 1, 0, __uuidof(IAudioProcessingObject),
	(APO_FLAG)(APO_FLAG_SAMPLESPERFRAME_MUST_MATCH | APO_FLAG_FRAMESPERSECOND_MUST_MATCH | APO_FLAG_BITSPERSAMPLE_MUST_MATCH | APO_FLAG_INPLACE));

MuteGuardAPO::MuteGuardAPO(IUnknown* pUnkOuter)
	: CBaseAudioProcessingObject(regProperties)
{
	refCount = 1;
	if (pUnkOuter != NULL)
		this->pUnkOuter = pUnkOuter;
	else
		this->pUnkOuter = reinterpret_cast<IUnknown*>(static_cast<INonDelegatingUnknown*>(this));

	InterlockedIncrement(&instCount);
	TraceApo("Construct", S_OK);
}

MuteGuardAPO::~MuteGuardAPO()
{
	InterlockedDecrement(&instCount);
}

HRESULT MuteGuardAPO::QueryInterface(const IID& iid, void** ppv)
{
	return pUnkOuter->QueryInterface(iid, ppv);
}

ULONG MuteGuardAPO::AddRef()
{
	return pUnkOuter->AddRef();
}

ULONG MuteGuardAPO::Release()
{
	return pUnkOuter->Release();
}

HRESULT MuteGuardAPO::GetLatency(HNSTIME* pTime)
{
	if (!pTime)
		return E_POINTER;

	if (!m_bIsLocked)
		return APOERR_ALREADY_UNLOCKED;

	*pTime = noise.active() ? 200000 : 0;

	return S_OK;
}

HRESULT MuteGuardAPO::Initialize(UINT32 cbDataSize, BYTE* pbyData)
{
	TraceApo("Initialize entered", S_OK);
	if ((NULL == pbyData) && (0 != cbDataSize))
		return E_INVALIDARG;
	if ((NULL != pbyData) && (0 == cbDataSize))
		return E_POINTER;
	if (cbDataSize < sizeof(APOInitSystemEffects))
		return E_INVALIDARG;

	const HRESULT result = CBaseAudioProcessingObject::Initialize(cbDataSize, pbyData);
	TraceApo("Initialize finished", result);
	return result;
}

HRESULT MuteGuardAPO::IsInputFormatSupported(IAudioMediaType* pOutputFormat,
	IAudioMediaType* pRequestedInputFormat, IAudioMediaType** ppSupportedInputFormat)
{
	if (!pRequestedInputFormat)
		return E_POINTER;

	UNCOMPRESSEDAUDIOFORMAT inFormat;
	HRESULT hr = pRequestedInputFormat->GetUncompressedAudioFormat(&inFormat);
	if (FAILED(hr))
	{
		return hr;
	}

	UNCOMPRESSEDAUDIOFORMAT outFormat;
	hr = pOutputFormat->GetUncompressedAudioFormat(&outFormat);
	if (FAILED(hr))
	{
		return hr;
	}

	hr = CBaseAudioProcessingObject::IsInputFormatSupported(pOutputFormat, pRequestedInputFormat, ppSupportedInputFormat);

	return hr;
}

HRESULT MuteGuardAPO::LockForProcess(UINT32 u32NumInputConnections,
	APO_CONNECTION_DESCRIPTOR** ppInputConnections, UINT32 u32NumOutputConnections,
	APO_CONNECTION_DESCRIPTOR** ppOutputConnections)
{
	HRESULT hr;

	UNCOMPRESSEDAUDIOFORMAT outFormat;
	hr = ppOutputConnections[0]->pFormat->GetUncompressedAudioFormat(&outFormat);
	if (FAILED(hr))
		return hr;

	hr = CBaseAudioProcessingObject::LockForProcess(u32NumInputConnections, ppInputConnections,
			u32NumOutputConnections, ppOutputConnections);
	if (FAILED(hr)) {
		TraceApo("LockForProcess failed", hr);
		return hr;
	}

	channelCount = outFormat.dwSamplesPerFrame;
	effects = Effects{};
	wchar_t configPath[MAX_PATH] = {};
	const DWORD pathLength = GetEnvironmentVariableW(L"ProgramData", configPath, MAX_PATH);
	if (pathLength > 0 && pathLength < MAX_PATH &&
		wcscat_s(configPath, L"\\MuteGuard\\APO-test\\effects.ini") == 0) {
		effects.gate = GetPrivateProfileIntW(L"Effects", L"Gate", 0, configPath) == 1;
		effects.compressor = GetPrivateProfileIntW(L"Effects", L"Compressor", 0, configPath) == 1;
		effects.gateThresholdDb = ReadBounded(L"GateThresholdDb", -45, -80, -10, configPath);
		effects.gateHoldMs = ReadBounded(L"GateHoldMs", 100, 0, 500, configPath);
		effects.gateReleaseMs = ReadBounded(L"GateReleaseMs", 150, 10, 1000, configPath);
		effects.compressorThresholdDb = ReadBounded(L"CompressorThresholdDb", -18, -40, 0, configPath);
		effects.compressorRatio = ReadBounded(L"CompressorRatioTenths", 25, 10, 100, configPath) / 10.0f;
		effects.compressorAttackMs = ReadBounded(L"CompressorAttackMs", 10, 1, 100, configPath);
		effects.compressorReleaseMs = ReadBounded(L"CompressorReleaseMs", 150, 20, 1000, configPath);
		const int noiseStrength = ReadBounded(L"NoiseStrength", 100, 0, 100, configPath);
		// shortcut: RNNoise needs 48 kHz; add resampling if other capture formats must be supported.
		if (outFormat.fFramesPerSecond == 48000.0f &&
			GetPrivateProfileIntW(L"Effects", L"Noise", 0, configPath) == 1 && noiseStrength > 0) {
			wchar_t modelPath[MAX_PATH] = {};
			if (wcscpy_s(modelPath, configPath) == 0) {
				wchar_t* filename = wcsrchr(modelPath, L'\\');
				if (filename && wcscpy_s(filename + 1, MAX_PATH - (filename + 1 - modelPath), L"weights_blob.bin") == 0)
					noise.prepare(modelPath, channelCount, noiseStrength / 100.0f);
			}
		}
	}
	effects.reset(outFormat.fFramesPerSecond);
	InterlockedExchange(&processedBuffers, 0);
	TraceApo("LockForProcess ready", S_OK);

	return hr;
}

HRESULT MuteGuardAPO::UnlockForProcess()
{
	TraceApo("UnlockForProcess", S_OK, InterlockedCompareExchange(&processedBuffers, 0, 0));
	noise.reset();
	return CBaseAudioProcessingObject::UnlockForProcess();
}

#pragma AVRT_CODE_BEGIN
void MuteGuardAPO::APOProcess(UINT32 u32NumInputConnections,
	APO_CONNECTION_PROPERTY** ppInputConnections, UINT32 u32NumOutputConnections,
	APO_CONNECTION_PROPERTY** ppOutputConnections)
{
	InterlockedIncrement(&processedBuffers);
	switch (ppInputConnections[0]->u32BufferFlags)
	{
	case BUFFER_VALID:
		{
			float* inputFrames = reinterpret_cast<float*>(ppInputConnections[0]->pBuffer);
			float* outputFrames = reinterpret_cast<float*>(ppOutputConnections[0]->pBuffer);

			const unsigned frames = ppInputConnections[0]->u32ValidFrameCount;
			if (noise.active()) noise.process(inputFrames, outputFrames, frames);
			else if (inputFrames != outputFrames) memcpy(outputFrames, inputFrames, frames * channelCount * sizeof(float));
			effects.process(outputFrames, outputFrames, frames, channelCount);
			ppOutputConnections[0]->u32ValidFrameCount = ppInputConnections[0]->u32ValidFrameCount;
			ppOutputConnections[0]->u32BufferFlags = ppInputConnections[0]->u32BufferFlags;

			break;
		}
	case BUFFER_SILENT:
		ppOutputConnections[0]->u32ValidFrameCount = ppInputConnections[0]->u32ValidFrameCount;
		ppOutputConnections[0]->u32BufferFlags = ppInputConnections[0]->u32BufferFlags;

		break;
	}
}
#pragma AVRT_CODE_END

HRESULT MuteGuardAPO::NonDelegatingQueryInterface(const IID& iid, void** ppv)
{
	if (iid == __uuidof(IUnknown))
		*ppv = static_cast<INonDelegatingUnknown*>(this);
	else if (iid == __uuidof(IAudioProcessingObject))
		*ppv = static_cast<IAudioProcessingObject*>(this);
	else if (iid == __uuidof(IAudioProcessingObjectRT))
		*ppv = static_cast<IAudioProcessingObjectRT*>(this);
	else if (iid == __uuidof(IAudioProcessingObjectConfiguration))
		*ppv = static_cast<IAudioProcessingObjectConfiguration*>(this);
	else if (iid == __uuidof(IAudioSystemEffects))
		*ppv = static_cast<IAudioSystemEffects*>(this);
	else
	{
		*ppv = NULL;
		return E_NOINTERFACE;
	}

	reinterpret_cast<IUnknown*>(*ppv)->AddRef();
	return S_OK;
}

ULONG MuteGuardAPO::NonDelegatingAddRef()
{
	return InterlockedIncrement(&refCount);
}

ULONG MuteGuardAPO::NonDelegatingRelease()
{
	if (InterlockedDecrement(&refCount) == 0)
	{
		delete this;
		return 0;
	}

	return refCount;
}
