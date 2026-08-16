#pragma once

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
