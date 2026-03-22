#pragma once

#include "inet/linklayer/ieee80211/mac/coordinationfunction/Hcf.h"

namespace veins_qos::mac {

class V2xEdcaFsmController;

class V2xHcf : public inet::ieee80211::Hcf
{
  protected:
    bool adaptiveBlocking = true;
    omnetpp::simtime_t blockDuration = SIMTIME_ZERO;
    omnetpp::simtime_t maxContinuousBlock = SIMTIME_ZERO;
    int voQueueThreshold = 1;
    omnetpp::cMessage *beRetryTimer = nullptr;

    V2xEdcaFsmController *fsmController = nullptr;

  protected:
    inet::ieee80211::AccessCategory classifyAccessCategory(const inet::Ptr<const inet::ieee80211::Ieee80211DataOrMgmtHeader>& header) const;
    bool hasBeQueuePressure() const;
    bool hasVoQueuePressure() const;
    bool isReceivedVoDataForUs(const inet::Ptr<const inet::ieee80211::Ieee80211MacHeader>& header) const;
    void maybeRequestChannelAccess(inet::ieee80211::AccessCategory ac);
    void scheduleBeRetry();

    virtual void initialize(int stage) override;
    virtual void handleMessage(omnetpp::cMessage *msg) override;
    virtual void processUpperFrame(inet::Packet *packet, const inet::Ptr<const inet::ieee80211::Ieee80211DataOrMgmtHeader>& header) override;
    virtual void processLowerFrame(inet::Packet *packet, const inet::Ptr<const inet::ieee80211::Ieee80211MacHeader>& header) override;
    virtual void channelGranted(inet::ieee80211::IChannelAccess *channelAccess) override;
    virtual void transmissionComplete(inet::Packet *packet, const inet::Ptr<const inet::ieee80211::Ieee80211MacHeader>& header) override;

  public:
    virtual ~V2xHcf() override;
};

} // namespace veins_qos::mac
