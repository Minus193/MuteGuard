#include "stdafx.h"

#include "MuteGuardAPO.h"
#include "processing.h"

using namespace std;

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
	if ((NULL == pbyData) && (0 != cbDataSize))
		return E_INVALIDARG;
	if ((NULL != pbyData) && (0 == cbDataSize))
		return E_POINTER;
	if (cbDataSize != sizeof(APOInitSystemEffects))
		return E_INVALIDARG;

	return S_OK;
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
	if (FAILED(hr))
		return hr;

	channelCount = outFormat.dwSamplesPerFrame;
	wchar_t configPath[MAX_PATH] = {};
	const DWORD pathLength = GetEnvironmentVariableW(L"ProgramData", configPath, MAX_PATH);
	if (pathLength > 0 && pathLength < MAX_PATH &&
		wcscat_s(configPath, L"\\MuteGuard\\APO-test\\effects.ini") == 0) {
		effects.gate = GetPrivateProfileIntW(L"Effects", L"Gate", 0, configPath) == 1;
		effects.compressor = GetPrivateProfileIntW(L"Effects", L"Compressor", 0, configPath) == 1;
		// shortcut: RNNoise needs 48 kHz; add resampling if other capture formats must be supported.
		if (outFormat.fFramesPerSecond == 48000.0f &&
			GetPrivateProfileIntW(L"Effects", L"Noise", 0, configPath) == 1) {
			wchar_t modelPath[MAX_PATH] = {};
			if (wcscpy_s(modelPath, configPath) == 0) {
				wchar_t* filename = wcsrchr(modelPath, L'\\');
				if (filename && wcscpy_s(filename + 1, MAX_PATH - (filename + 1 - modelPath), L"weights_blob.bin") == 0)
					noise.prepare(modelPath, channelCount);
			}
		}
	}
	effects.reset(outFormat.fFramesPerSecond);

	return hr;
}

HRESULT MuteGuardAPO::UnlockForProcess()
{
	noise.reset();
	return CBaseAudioProcessingObject::UnlockForProcess();
}

#pragma AVRT_CODE_BEGIN
void MuteGuardAPO::APOProcess(UINT32 u32NumInputConnections,
	APO_CONNECTION_PROPERTY** ppInputConnections, UINT32 u32NumOutputConnections,
	APO_CONNECTION_PROPERTY** ppOutputConnections)
{
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
