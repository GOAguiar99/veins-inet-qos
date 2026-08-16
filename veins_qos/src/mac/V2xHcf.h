#pragma once

#include <map>

#include "inet/linklayer/common/MacAddress.h"
#include "inet/linklayer/ieee80211/mac/coordinationfunction/Hcf.h"

namespace veins_qos::mac {

class V2xEdcaFsmController;

class V2xHcf : public inet::ieee80211::Hcf
{
  protected:
    bool adaptiveBlocking = true;
    bool emergencyPreemption = false;
    omnetpp::simtime_t blockDuration = SIMTIME_ZERO;
    omnetpp::simtime_t maxContinuousBlock = SIMTIME_ZERO;
    int voQueueThreshold = 1;
    omnetpp::cMessage *beRetryTimer = nullptr;

    V2xEdcaFsmController *fsmController = nullptr;
    omnetpp::simsignal_t beDroppedWhileBlockedSignal = omnetpp::simsignal_t();
    omnetpp::simsignal_t beGrantSuppressedWhileBlockedSignal = omnetpp::simsignal_t();
    omnetpp::simsignal_t voProtectionActivationSignal = omnetpp::simsignal_t();
    long beDroppedWhileBlockedCount = 0;
    long beGrantSuppressedWhileBlockedCount = 0;
    long voProtectionActivationCount = 0;

    // Predictive periodic protection (optional): learn the crash VO burst
    // cadence from overheard VO frames and block BE right before the next
    // predicted burst, instead of only reacting once a copy was decoded.
    bool predictiveBlocking = false;
    omnetpp::simtime_t predictiveLead = SIMTIME_ZERO;
    omnetpp::simtime_t predictiveWindow = SIMTIME_ZERO;
    omnetpp::simtime_t predictiveMinGap = SIMTIME_ZERO;
    omnetpp::simtime_t predictiveMinPeriod = SIMTIME_ZERO;
    omnetpp::simtime_t predictiveMaxPeriod = SIMTIME_ZERO;

    struct VoPredictorTrack {
        omnetpp::simtime_t lastFrameAt = -1;
        omnetpp::simtime_t lastBurstAt = -1;
        omnetpp::simtime_t period = -1;
        omnetpp::simtime_t preBlockAt = -1;
    };
    std::map<inet::MacAddress, VoPredictorTrack> voPredictorTracks;
    omnetpp::cMessage *predictTimer = nullptr;
    omnetpp::simsignal_t voPredictiveBlockSignal = omnetpp::simsignal_t();
    long voPredictiveBlockCount = 0;

  protected:
    inet::ieee80211::AccessCategory classifyAccessCategory(const inet::Ptr<const inet::ieee80211::Ieee80211DataOrMgmtHeader>& header) const;
    bool hasBeQueuePressure() const;
    bool hasVoQueuePressure() const;
    bool hasAnyVoQueuePressure() const;
    bool isReceivedVoDataForUs(const inet::Ptr<const inet::ieee80211::Ieee80211MacHeader>& header) const;
    bool isEmergencyBlockingActive() const;
    void activateVoProtection(omnetpp::simtime_t duration);
    void dropBeWhileBlocked(inet::Packet *packet);
    void maybeRequestChannelAccess(inet::ieee80211::AccessCategory ac);
    void scheduleBeRetry();
    void feedVoPredictor(const inet::MacAddress& source, omnetpp::simtime_t now);
    void onPredictTimer();
    void refreshPredictTimer();

    virtual void initialize(int stage) override;
    virtual void finish() override;
    virtual void handleMessage(omnetpp::cMessage *msg) override;
    virtual void processUpperFrame(inet::Packet *packet, const inet::Ptr<const inet::ieee80211::Ieee80211DataOrMgmtHeader>& header) override;
    virtual void processLowerFrame(inet::Packet *packet, const inet::Ptr<const inet::ieee80211::Ieee80211MacHeader>& header) override;
    virtual void channelGranted(inet::ieee80211::IChannelAccess *channelAccess) override;
    virtual void transmissionComplete(inet::Packet *packet, const inet::Ptr<const inet::ieee80211::Ieee80211MacHeader>& header) override;

  public:
    virtual ~V2xHcf() override;
};

} // namespace veins_qos::mac
