#include "mac/V2xIeee80211Mac.h"

#include "inet/common/Simsignals.h"
#include "inet/linklayer/common/UserPriorityTag_m.h"
#include "inet/linklayer/ieee80211/mac/Ieee80211Frame_m.h"

namespace veins_qos::mac {

using namespace inet;
using namespace inet::ieee80211;

Define_Module(V2xIeee80211Mac);

std::string V2xIeee80211Mac::acSuffix(TrackedAc ac)
{
    switch (ac) {
        case TrackedAc::BK: return "Bk";
        case TrackedAc::BE: return "Be";
        case TrackedAc::VI: return "Vi";
        case TrackedAc::VO: return "Vo";
        case TrackedAc::UNCLASSIFIED: return "Unclassified";
        default: return "Unknown";
    }
}

std::string V2xIeee80211Mac::reasonSuffix(int reason)
{
    switch (static_cast<PacketDropReason>(reason)) {
        case ADDRESS_RESOLUTION_FAILED: return "AddressResolutionFailed";
        case FORWARDING_DISABLED: return "ForwardingDisabled";
        case HOP_LIMIT_REACHED: return "HopLimitReached";
        case INCORRECTLY_RECEIVED: return "IncorrectlyReceived";
        case INTERFACE_DOWN: return "InterfaceDown";
        case NO_CARRIER: return "NoCarrier";
        case NO_INTERFACE_FOUND: return "NoInterfaceFound";
        case NO_ROUTE_FOUND: return "NoRouteFound";
        case NOT_ADDRESSED_TO_US: return "NotAddressedToUs";
        case QUEUE_OVERFLOW: return "QueueOverflow";
        case RETRY_LIMIT_REACHED: return "RetryLimitReached";
        case LIFETIME_EXPIRED: return "LifetimeExpired";
        case CONGESTION: return "Congestion";
        case NO_PROTOCOL_FOUND: return "NoProtocolFound";
        case NO_PORT_FOUND: return "NoPortFound";
        case DUPLICATE_DETECTED: return "DuplicateDetected";
        case OTHER_PACKET_DROP: return "Other";
        default: return "Reason" + std::to_string(reason);
    }
}

AccessCategory V2xIeee80211Mac::mapTidToAc(int tid)
{
    switch (tid) {
        case 1:
        case 2: return AC_BK;
        case 0:
        case 3: return AC_BE;
        case 4:
        case 5: return AC_VI;
        case 6:
        case 7: return AC_VO;
        default: return AC_BE;
    }
}

V2xIeee80211Mac::TrackedAc V2xIeee80211Mac::inferAccessCategory(const Packet *packet) const
{
    if (packet == nullptr)
        return TrackedAc::UNCLASSIFIED;

    if (auto userPriorityReq = packet->findTag<UserPriorityReq>()) {
        switch (mapTidToAc(userPriorityReq->getUserPriority())) {
            case AC_BK: return TrackedAc::BK;
            case AC_BE: return TrackedAc::BE;
            case AC_VI: return TrackedAc::VI;
            case AC_VO: return TrackedAc::VO;
            default: return TrackedAc::UNCLASSIFIED;
        }
    }

    if (auto userPriorityInd = packet->findTag<UserPriorityInd>()) {
        switch (mapTidToAc(userPriorityInd->getUserPriority())) {
            case AC_BK: return TrackedAc::BK;
            case AC_BE: return TrackedAc::BE;
            case AC_VI: return TrackedAc::VI;
            case AC_VO: return TrackedAc::VO;
            default: return TrackedAc::UNCLASSIFIED;
        }
    }

    if (!packet->hasAtFront<Ieee80211MacHeader>())
        return TrackedAc::UNCLASSIFIED;

    auto macHeader = packet->peekAtFront<Ieee80211MacHeader>();
    if (auto dataHeader = dynamicPtrCast<const Ieee80211DataHeader>(macHeader)) {
        if (dataHeader->getType() == ST_DATA_WITH_QOS) {
            switch (mapTidToAc(dataHeader->getTid())) {
                case AC_BK: return TrackedAc::BK;
                case AC_BE: return TrackedAc::BE;
                case AC_VI: return TrackedAc::VI;
                case AC_VO: return TrackedAc::VO;
                default: return TrackedAc::UNCLASSIFIED;
            }
        }
        return TrackedAc::UNCLASSIFIED;
    }

    if (dynamicPtrCast<const Ieee80211MgmtHeader>(macHeader))
        return TrackedAc::VO;

    return TrackedAc::UNCLASSIFIED;
}

void V2xIeee80211Mac::countPacketDrop(const Packet *packet, const PacketDropDetails *details)
{
    auto ac = inferAccessCategory(packet);
    int acIndex = trackedAcIndex(ac);
    packetDropCountByAc[acIndex]++;

    int reason = details != nullptr ? static_cast<int>(details->getReason()) : static_cast<int>(OTHER_PACKET_DROP);
    auto &byAc = packetDropCountByReasonAndAc[reason];
    byAc[acIndex]++;
}

void V2xIeee80211Mac::subscribePacketDropSignalsRecursively(cModule *module)
{
    if (module == nullptr)
        return;

    module->subscribe(packetDroppedSignal, this);
    for (cModule::SubmoduleIterator it(module); !it.end(); ++it)
        subscribePacketDropSignalsRecursively(*it);
}

void V2xIeee80211Mac::recordPacketDropScalars()
{
    // Per-AC totals.
    for (int acIndex = 0; acIndex < static_cast<int>(kTrackedAcCount); acIndex++) {
        auto ac = static_cast<TrackedAc>(acIndex);
        std::string metricName = "packetDropAc" + acSuffix(ac) + "Count";
        recordScalar(metricName.c_str(), packetDropCountByAc[acIndex]);
    }

    // Per-AC and per-drop-reason counters.
    for (const auto& entry : packetDropCountByReasonAndAc) {
        int reason = entry.first;
        const auto& byAc = entry.second;
        std::string reasonName = reasonSuffix(reason);
        for (int acIndex = 0; acIndex < static_cast<int>(kTrackedAcCount); acIndex++) {
            auto ac = static_cast<TrackedAc>(acIndex);
            std::string metricName = "packetDropAc" + acSuffix(ac) + "Reason" + reasonName + "Count";
            recordScalar(metricName.c_str(), byAc[acIndex]);
        }
    }
}

void V2xIeee80211Mac::initialize(int stage)
{
    Ieee80211Mac::initialize(stage);

    if (stage == INITSTAGE_LINK_LAYER) {
        packetDropCountByAc.fill(0);
        packetDropCountByReasonAndAc.clear();
        subscribePacketDropSignalsRecursively(this);
    }
}

void V2xIeee80211Mac::finish()
{
    recordPacketDropScalars();
}

void V2xIeee80211Mac::receiveSignal(cComponent *source, simsignal_t signalID, cObject *obj, cObject *details)
{
    Enter_Method("%s", cComponent::getSignalName(signalID));

    if (signalID == packetDroppedSignal) {
        auto packet = dynamic_cast<Packet *>(obj);
        auto packetDropDetails = dynamic_cast<PacketDropDetails *>(details);
        countPacketDrop(packet, packetDropDetails);
        return;
    }

    MacProtocolBase::receiveSignal(source, signalID, obj, details);
}

} // namespace veins_qos::mac
