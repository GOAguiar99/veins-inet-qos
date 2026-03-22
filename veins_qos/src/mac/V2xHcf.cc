#include "mac/V2xHcf.h"

#include <algorithm>

#include "mac/V2xEdcaFsmController.h"
#include "inet/linklayer/ieee80211/mac/Ieee80211Frame_m.h"

namespace veins_qos::mac {

using namespace inet;
using namespace inet::ieee80211;

Define_Module(V2xHcf);

V2xHcf::~V2xHcf()
{
    cancelAndDelete(beRetryTimer);
}

void V2xHcf::initialize(int stage)
{
    Hcf::initialize(stage);

    if (stage == INITSTAGE_LOCAL) {
        adaptiveBlocking = par("adaptiveBlocking").boolValue();
        blockDuration = par("blockDuration");
        maxContinuousBlock = par("maxContinuousBlock");
        voQueueThreshold = std::max(1, static_cast<int>(par("voQueueThreshold").intValue()));
        if (maxContinuousBlock > SIMTIME_ZERO && blockDuration > maxContinuousBlock)
            blockDuration = maxContinuousBlock;

        fsmController = check_and_cast<V2xEdcaFsmController *>(getSubmodule("FSMController"));
        beRetryTimer = new cMessage("beRetryTimer");

        EV_INFO << "V2xHcf init"
                << " adaptiveBlocking=" << adaptiveBlocking
                << " blockDuration=" << blockDuration
                << " maxContinuousBlock=" << maxContinuousBlock
                << " voQueueThreshold=" << voQueueThreshold
                << endl;
    }
}

AccessCategory V2xHcf::classifyAccessCategory(const Ptr<const Ieee80211DataOrMgmtHeader>& header) const
{
    if (dynamicPtrCast<const Ieee80211MgmtHeader>(header))
        return AccessCategory::AC_VO;
    if (auto dataHeader = dynamicPtrCast<const Ieee80211DataHeader>(header))
        return edca->classifyFrame(dataHeader);

    throw cRuntimeError("Unknown upper frame type");
}

bool V2xHcf::hasBeQueuePressure() const
{
    auto beQueue = edca->getEdcaf(AccessCategory::AC_BE)->getPendingQueue();
    return beQueue != nullptr && !beQueue->isEmpty();
}

bool V2xHcf::hasVoQueuePressure() const
{
    auto voQueue = edca->getEdcaf(AccessCategory::AC_VO)->getPendingQueue();
    return voQueue != nullptr && voQueue->getNumPackets() >= voQueueThreshold;
}

bool V2xHcf::isReceivedVoDataForUs(const Ptr<const Ieee80211MacHeader>& header) const
{
    if (!isForUs(header))
        return false;

    auto dataHeader = dynamicPtrCast<const Ieee80211DataHeader>(header);
    if (dataHeader == nullptr)
        return false;

    // Reuse EDCA classification to keep RX-trigger logic aligned with the active QoS mapping.
    return edca->classifyFrame(dataHeader) == AccessCategory::AC_VO;
}

void V2xHcf::maybeRequestChannelAccess(AccessCategory ac)
{
    auto owner = edca->getChannelOwner();
    if (owner == nullptr || owner->getAccessCategory() != ac)
        edca->requestChannelAccess(ac, this);
}

void V2xHcf::scheduleBeRetry()
{
    if (getSimulation()->getContextModule() != this) {
        Enter_Method("scheduleBeRetry");
    }

    if (beRetryTimer == nullptr || !hasBeQueuePressure())
        return;

    simtime_t retryAt = simTime();
    if (adaptiveBlocking && fsmController != nullptr && fsmController->isBeBlocked()) {
        retryAt = fsmController->getBlockingUntil();
        if (retryAt <= simTime())
            retryAt = simTime() + SimTime(1, SIMTIME_US);
    }

    if (beRetryTimer->isScheduled())
        rescheduleAt(retryAt, beRetryTimer);
    else
        scheduleAt(retryAt, beRetryTimer);
}

void V2xHcf::handleMessage(cMessage *msg)
{
    if (msg == beRetryTimer) {
        if (adaptiveBlocking && fsmController != nullptr && fsmController->isBeBlocked()) {
            scheduleBeRetry();
            return;
        }

        if (hasBeQueuePressure())
            maybeRequestChannelAccess(AccessCategory::AC_BE);
        return;
    }

    Hcf::handleMessage(msg);
}

void V2xHcf::processUpperFrame(Packet *packet, const Ptr<const Ieee80211DataOrMgmtHeader>& header)
{
    Enter_Method("processUpperFrame(%s)", packet->getName());
    take(packet);

    auto ac = classifyAccessCategory(header);
    auto pendingQueue = edca->getEdcaf(ac)->getPendingQueue();
    pendingQueue->enqueuePacket(packet);

    if (pendingQueue->isEmpty())
        return;

    if (adaptiveBlocking && fsmController != nullptr) {
        if (ac == AccessCategory::AC_VO && hasVoQueuePressure()) {
            fsmController->onVoDemandDetected(blockDuration);
            maybeRequestChannelAccess(AccessCategory::AC_VO);
            return;
        }

        if (ac == AccessCategory::AC_BE && fsmController->isBeBlocked()) {
            EV_DETAIL << "Suppressing BE request while FSM is blocking/sending" << endl;
            scheduleBeRetry();
            return;
        }
    }

    maybeRequestChannelAccess(ac);
}

void V2xHcf::channelGranted(IChannelAccess *channelAccess)
{
    Enter_Method("channelGranted");

    auto edcaf = check_and_cast<Edcaf *>(channelAccess);
    if (adaptiveBlocking && fsmController != nullptr && edcaf->getAccessCategory() == AccessCategory::AC_VO)
        fsmController->onVoTransmissionStart();

    Hcf::channelGranted(channelAccess);
}

void V2xHcf::transmissionComplete(Packet *packet, const Ptr<const Ieee80211MacHeader>& header)
{
    Enter_Method("transmissionComplete");

    bool voDataTxContext = false;
    auto owner = edca->getChannelOwner();
    if (owner != nullptr && owner->getAccessCategory() == AccessCategory::AC_VO) {
        auto dataOrMgmt = dynamicPtrCast<const Ieee80211DataOrMgmtHeader>(header);
        voDataTxContext = dataOrMgmt != nullptr;
    }

    Hcf::transmissionComplete(packet, header);

    if (adaptiveBlocking && fsmController != nullptr && voDataTxContext) {
        bool hasPendingVo = hasVoQueuePressure();
        fsmController->onVoTransmissionEnd(hasPendingVo);
        if (hasPendingVo)
            fsmController->onVoDemandDetected(blockDuration);
        else if (hasBeQueuePressure())
            scheduleBeRetry();
    }
}

void V2xHcf::processLowerFrame(Packet *packet, const Ptr<const Ieee80211MacHeader>& header)
{
    Enter_Method("processLowerFrame(%s)", packet->getName());

    // Received VO addressed to this node also extends alert mode, reducing BE contention during crash traffic.
    if (adaptiveBlocking && fsmController != nullptr && isReceivedVoDataForUs(header)) {
        fsmController->onVoDemandDetected(blockDuration);
        if (hasBeQueuePressure())
            scheduleBeRetry();
    }

    Hcf::processLowerFrame(packet, header);
}

} // namespace veins_qos::mac
