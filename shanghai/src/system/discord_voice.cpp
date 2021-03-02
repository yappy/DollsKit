#include "discord_voice.h"

namespace shanghai {
namespace system {

void SafeVoiceContext::Set(MyDiscordClient *p)
{
	std::lock_guard<decltype(mtx)> lock(mtx);
	client = p;
}

void SafeVoiceContext::Set(VoiceEventHandler *p)
{
	std::lock_guard<decltype(mtx)> lock(mtx);
	eh = p;
}

void SafeVoiceContext::Set(WavSource *p)
{
	std::lock_guard<decltype(mtx)> lock(mtx);
	src = p;
}

void SafeVoiceContext::ClearClient() noexcept
{
	std::lock_guard<decltype(mtx)> lock(mtx);
	client = nullptr;
}

void SafeVoiceContext::ClearEventHandler() noexcept
{
	std::lock_guard<decltype(mtx)> lock(mtx);
	eh = nullptr;
}

void SafeVoiceContext::ClearSource() noexcept
{
	std::lock_guard<decltype(mtx)> lock(mtx);
	src = nullptr;
}

VoiceEventHandler::~VoiceEventHandler()
{
	m_ctx->ClearEventHandler();
}

void VoiceEventHandler::onReady(SleepyDiscord::VoiceConnection &vc)
{
	if (OnReady) OnReady(m_ctx, vc);
}

void VoiceEventHandler::onSpeaking(SleepyDiscord::VoiceConnection &vc)
{
	if (OnSpeaking) OnSpeaking(m_ctx, vc);
}

void VoiceEventHandler::onEndSpeaking(SleepyDiscord::VoiceConnection &vc)
{
	if (OnEndSpeaking) OnEndSpeaking(m_ctx, vc);
}

WavSource::WavSource(const SafeVoiceContextPtr &ctx,
	const std::string &path, int volume) :
	m_ctx(ctx), m_volume(volume)
{
	m_fp.reset(std::fopen(path.c_str(), "rb"));
	if (m_fp == nullptr) {
		throw FileError("file open failed: " + path);
	}
	// TODO: parse wav header
	m_buf.resize(BufSize);
}

WavSource::~WavSource()
{
	m_ctx->ClearSource();
}

void WavSource::read(
	SleepyDiscord::AudioTransmissionDetails &details,
	SleepyDiscord::AudioSample *&buffer, std::size_t &length) noexcept
{
	if (m_cancel.load()) {
		length = 0;
		return;
	}
	std::size_t rsize = fread(m_buf.data(), sizeof(SleepyDiscord::AudioSample),
		BufSize, m_fp.get());
	// 半端なサイズを返すとライブラリがクラッシュするので足りない場合は無音で埋める
	for (auto i = rsize; i < m_buf.size(); i++) {
		m_buf[i] = 0;
	}
	int32_t volume = m_volume.load();
	if (volume != VolumeMax) {
		for (auto &sample : m_buf) {
			int32_t org = sample;
			org = org * volume / VolumeMax;
			sample = org;
		}
	}

	buffer = m_buf.data();
	length = (rsize > 0) ? BufSize : 0;
}

void WavSource::SetVolume(int volume)
{
	m_volume.store(volume);
}

void WavSource::Cancel()
{
	m_cancel.store(true);
}

}	// namespace system
}	// namespace shanghai
