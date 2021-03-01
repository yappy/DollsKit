#ifndef SHANGHAI_SYSTEM_DISCORD_VOICE_H
#define SHANGHAI_SYSTEM_DISCORD_VOICE_H

#include <sleepy_discord/sleepy_discord.h>
#include <atomic>
#include <mutex>
#include "../util.h"

namespace shanghai {
namespace system {

class MyDiscordClient;
class VoiceEventHandler;
class WavSource;

// sleepy-discord の寿命管理が危険なので何とかするクラス
// Client, VoiceEventHandler, AudioSource から shared_ptr<> される
// それぞれのデストラクタの先頭で mtx を取りながら自身へのポインタを無効化する
struct SafeVoiceContext {
	std::mutex mtx;
	MyDiscordClient *client = nullptr;
	VoiceEventHandler *eh = nullptr;
	WavSource *src = nullptr;

	SafeVoiceContext() = default;
	SafeVoiceContext(const SafeVoiceContext &) = delete;
	SafeVoiceContext & operator=(const SafeVoiceContext &) = delete;
	~SafeVoiceContext() = default;

	// スレッドセーフにポインタをセットする
	void Set(MyDiscordClient *p);
	void Set(VoiceEventHandler *p);
	void Set(WavSource *p);
	// スレッドセーフにポインタをクリアする
	// デストラクタでこれを呼んで自分を消すこと
	void ClearClient() noexcept;
	void ClearEventHandler() noexcept;
	void ClearSource() noexcept;

	// ロックを取りポインタが有効ならば、ロックを取ったまま処理を行う
	template <class F>
	void CallWithClient(F func)
	{
		std::lock_guard<decltype(mtx)> lock(mtx);
		if (client != nullptr) func(client);
	}
	template <class F>
	void CallWithEventHandler(F func)
	{
		std::lock_guard<decltype(mtx)> lock(mtx);
		if (eh != nullptr) func(eh);
	}
	template <class F>
	void CallWithSource(F func)
	{
		std::lock_guard<decltype(mtx)> lock(mtx);
		if (src != nullptr) func(src);
	}
};
using SafeVoiceContextPtr = std::shared_ptr<SafeVoiceContext>;

// SafeVoiceContext へ shared_ptr を持つイベントハンドラ
class VoiceEventHandler : public SleepyDiscord::BaseVoiceEventHandler
{
public:
	explicit VoiceEventHandler(const SafeVoiceContextPtr &ctx) : m_ctx(ctx)
	{}
	VoiceEventHandler(const VoiceEventHandler &) = delete;
	VoiceEventHandler & operator=(const VoiceEventHandler &) = delete;

	virtual ~VoiceEventHandler();

	using CallBack = std::function<void (
		const SafeVoiceContextPtr &,
		SleepyDiscord::VoiceConnection &)>;
	CallBack OnReady = nullptr;
	CallBack OnSpeaking = nullptr;
	CallBack OnEndSpeaking = nullptr;
	void onReady(SleepyDiscord::VoiceConnection &vc) override;
	void onSpeaking(SleepyDiscord::VoiceConnection &vc) override;
	void onEndSpeaking(SleepyDiscord::VoiceConnection &vc) override;

private:
	SafeVoiceContextPtr m_ctx;
};

// SafeVoiceContext へ shared_ptr を持つ Wave audio source
class WavSource : public SleepyDiscord::AudioPointerSource
{
public:
	WavSource(const SafeVoiceContextPtr &ctx, const std::string &path);
	WavSource(const WavSource &) = delete;
	WavSource & operator=(const WavSource &) = delete;

	virtual ~WavSource();

	void read(
		SleepyDiscord::AudioTransmissionDetails &details,
		SleepyDiscord::AudioSample *&buffer,
		std::size_t &length) noexcept override;
	void Cancel();

private:
	SafeVoiceContextPtr m_ctx;
	util::File m_fp;
	std::atomic_bool m_cancel = false;
	std::vector<SleepyDiscord::AudioSample> m_buf;
	static constexpr std::size_t BufSize =
		SleepyDiscord::AudioTransmissionDetails::proposedLength();
};

}	// namespace system
}	// namespace shanghai

#endif	// SHANGHAI_SYSTEM_DISCORD_VOICE_H
