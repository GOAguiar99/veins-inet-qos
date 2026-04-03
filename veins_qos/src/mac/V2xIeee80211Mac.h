#pragma once

#include <array>
#include <map>
#include <string>

#include "inet/linklayer/ieee80211/mac/Ieee80211Mac.h"

namespace veins_qos::mac {

class V2xIeee80211Mac : public inet::ieee80211::Ieee80211Mac
{
  protected:
    enum class TrackedAc : int { BK = 0, BE = 1, VI = 2, VO = 3, UNCLASSIFIED = 4 };
    static constexpr size_t kTrackedAcCount = 5;
    using AcCounters = std::array<long, kTrackedAcCount>;

    AcCounters packetDropCountByAc = {};
    std::map<int, AcCounters> packetDropCountByReasonAndAc;

  protected:
    static int trackedAcIndex(TrackedAc ac) { return static_cast<int>(ac); }
    static std::string acSuffix(TrackedAc ac);
    static std::string reasonSuffix(int reason);
    static inet::ieee80211::AccessCategory mapTidToAc(int tid);

    TrackedAc inferAccessCategory(const inet::Packet *packet) const;
    void countPacketDrop(const inet::Packet *packet, const inet::PacketDropDetails *details);
    void subscribePacketDropSignalsRecursively(omnetpp::cModule *module);
    void recordPacketDropScalars();

    virtual void initialize(int stage) override;
    virtual void finish() override;
    virtual void receiveSignal(omnetpp::cComponent *source, omnetpp::simsignal_t signalID, omnetpp::cObject *obj, omnetpp::cObject *details) override;
};

} // namespace veins_qos::mac
