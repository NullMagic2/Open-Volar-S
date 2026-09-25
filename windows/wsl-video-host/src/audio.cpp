// WSL-only PCM output. Audio is pulled by the Windows endpoint clock; no
// independent timer or unbounded queue is allowed between mpv and WASAPI.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <ksmedia.h>
#include <wrl/client.h>
#include <cstdint>
#include <cstdio>
#include <vector>
#include <algorithm>
using Microsoft::WRL::ComPtr;
using Transfer = int (*)(void *, unsigned char *, int);
static constexpr uint32_t MAGIC = 0x4153564f; // OVSA, little endian, protocol 1

extern "C" int ovs_audio_run(Transfer read, Transfer write, void *io) {
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(hr)) return -1;
    struct Cleanup { ~Cleanup() { CoUninitialize(); } } cleanup;
    uint32_t ready[2] = {MAGIC, 1};
    if (write(io, (unsigned char *)ready, sizeof ready) < 0) return -1;
    for (;;) {
        // A new SDL device (track/rate change) starts a new endpoint session.
        uint32_t config[2];
        if (read(io, (unsigned char *)config, sizeof config) < 0) return 0;
        unsigned channels = config[1];
        if (config[0] != 48000 || (channels != 1 && channels != 2 && channels != 6 && channels != 8)) return -1;
        ComPtr<IMMDeviceEnumerator> enumerator;
        ComPtr<IMMDevice> device;
        ComPtr<IAudioClient> client;
        ComPtr<IAudioRenderClient> render;
        HANDLE event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        struct Close { HANDLE h; ~Close() { if (h) CloseHandle(h); } } close{event};
        WAVEFORMATEXTENSIBLE format = {};
        format.Format = {WAVE_FORMAT_EXTENSIBLE, (WORD)channels, 48000,
            48000 * channels * 4, (WORD)(channels * 4), 32, 22};
        format.Samples.wValidBitsPerSample = 32;
        format.dwChannelMask = channels == 1 ? 4 : channels == 2 ? 3 : channels == 6 ? 0x3f : 0x63f;
        format.SubFormat = KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
        UINT32 capacity = 0;
        REFERENCE_TIME device_period = 0;
        auto setup = [&]() -> HRESULT {
            if (!event) return HRESULT_FROM_WIN32(GetLastError());
            HRESULT r = CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL, IID_PPV_ARGS(&enumerator));
            if (FAILED(r)) return r;
            r = enumerator->GetDefaultAudioEndpoint(eRender, eConsole, &device);
            if (FAILED(r)) return r;
            r = device->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, (void **)client.GetAddressOf());
            if (FAILED(r)) return r;
            r = client->Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK |
                AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                0, 0, &format.Format, nullptr);
            if (FAILED(r)) return r;
            r = client->SetEventHandle(event); if (FAILED(r)) return r;
            r = client->GetBufferSize(&capacity); if (FAILED(r)) return r;
            r = client->GetDevicePeriod(&device_period, nullptr); if (FAILED(r)) return r;
            return client->GetService(IID_PPV_ARGS(&render));
        };
        hr = setup();
        uint32_t period = (uint32_t)((device_period * 48000 + 9999999) / 10000000);
        if (SUCCEEDED(hr) && (period < 64 || period > 4096 || capacity < 2 * period)) hr = E_NOTIMPL;
        uint32_t answer[4] = {SUCCEEDED(hr) ? MAGIC : 0, 48000, channels, period};
        if (write(io, (unsigned char *)answer, sizeof answer) < 0) return 0;
        if (FAILED(hr)) {
            fprintf(stderr, "[DEBUG]: Windows WASAPI not available (0x%08lx); trying WSLg audio\n", (unsigned long)hr);
            continue;
        }
        fprintf(stderr, "[DEBUG]: audio output active: Windows WASAPI, 48000 Hz, %u channels, %u-frame period\n", channels, period);
        std::vector<unsigned char> pcm(period * channels * 4);
        // SDL's callback AO estimates two callback periods of latency. Maintain
        // exactly this target on the real endpoint, rather than adding another
        // Pulse/RDP queue. Windows owns the pacing, including device clock drift.
        bool running = false, closed = false;
        while (!closed) {
            UINT32 padding = 0;
            hr = client->GetCurrentPadding(&padding); if (FAILED(hr)) break;
            while (padding <= period) {
                uint32_t request = period, active = 0;
                if (write(io, (unsigned char *)&request, 4) < 0 || read(io, (unsigned char *)&active, 4) < 0) return 0;
                if (!active) { closed = true; break; }
                if (active != 1 || read(io, pcm.data(), (int)pcm.size()) < 0) return -1;
                BYTE *buffer = nullptr;
                hr = render->GetBuffer(period, &buffer); if (FAILED(hr)) break;
                memcpy(buffer, pcm.data(), pcm.size());
                hr = render->ReleaseBuffer(period, 0); if (FAILED(hr)) break;
                padding += period;
            }
            if (FAILED(hr) || closed) break;
            if (!running) { hr = client->Start(); if (FAILED(hr)) break; running = true; }
            if (WaitForSingleObject(event, 2000) != WAIT_OBJECT_0) { hr = E_FAIL; break; }
        }
        if (running) client->Stop();
        client->Reset();
        if (FAILED(hr)) {
            fprintf(stderr, "[ERROR]: Windows audio endpoint failed (0x%08lx); restart playback to reopen the output\n", (unsigned long)hr);
            return -1;
        }
    }
}
