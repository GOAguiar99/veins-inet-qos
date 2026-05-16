#include "mac/V2xHcf.h"

#include <algorithm>
#include <fstream>
#include <string>

#include "mac/V2xEdcaFsmController.h"
#include "inet/common/Simsignals.h"
#include "inet/linklayer/ieee80211/mac/Ieee80211Frame_m.h"

namespace veins_qos::mac {

using namespace inet;
using namespace inet::ieee80211;

Define_Module(V2xHcf);

namespace {
void agentDebugLog(const char *hypothesisId, const char *location, const char *message, const std::string& data)
{
    std::ofstream out("/home/goaguiar/master/master_veins/.cursor/debug-9574a1.log", std::ios::app);
    out << "{\"sessionId\":\"9574a1\",\"runId\":\"pre-fix\",\"hypothesisId\":\"" << hypothesisId
        << "\",\"location\":\"" << location << "\",\"message\":\"" << message
        << "\",\"data\":{" << data << "},\"timestamp\":" << static_cast<long long>(omnetpp::simTime().dbl() * 1000) << "}\n";
}
} // namespace

V2xHcf::~V2xHcf()
{
    cancelAndDelete(beRetryTimer);
}

void V2xHcf::initialize(int stage)
{
    Hcf::initialize(stage);

    if (stage == INITSTAGE_LOCAL) {
        adaptiveBlocking = par("adaptiveBlocking").boolValue();
        emergencyPreemption = par("emergencyPreemption").boolValue();
        blockDuration = par("blockDuration");
        maxContinuousBlock = par("maxContinuousBlock");
        voQueueThreshold = std::max(1, static_cast<int>(par("voQueueThreshold").intValue()));
        if (maxContinuousBlock > SIMTIME_ZERO && blockDuration > maxContinuousBlock)
            blockDuration = maxContinuousBlock;

        fsmController = check_and_cast<V2xEdcaFsmController *>(getSubmodule("FSMController"));
        beRetryTimer = new cMessage("beRetryTimer");
        beDroppedWhileBlockedSignal = registerSignal("beDroppedWhileBlocked");
        beGrantSuppressedWhileBlockedSignal = registerSignal("beGrantSuppressedWhileBlocked");
        voProtectionActivationSignal = registerSignal("voProtectionActivation");

        EV_INFO << "V2xHcf init"
                << " adaptiveBlocking=" << adaptiveBlocking
                << " emergencyPreemption=" << emergencyPreemption
                << " blockDuration=" << blockDuration
                << " maxContinuousBlock=" << maxContinuousBlock
                << " voQueueThreshold=" << voQueueThreshold
                << endl;
    }
}

void V2xHcf::finish()
{
    omnetpp::cSimpleModule::finish();
    recordScalar("beDroppedWhileBlockedCount", beDroppedWhileBlockedCount);
    recordScalar("beGrantSuppressedWhileBlockedCount", beGrantSuppressedWhileBlockedCount);
    recordScalar("voProtectionActivationCount", voProtectionActivationCount);
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

bool V2xHcf::hasAnyVoQueuePressure() const
{
    auto voQueue = edca->getEdcaf(AccessCategory::AC_VO)->getPendingQueue();
    return voQueue != nullptr && !voQueue->isEmpty();
}

bool V2xHcf::isReceivedVoDataForUs(const Ptr<const Ieee80211MacHeader>& header) const
{
    // Emergency protection must trigger on overheard VO crash alerts (multicast/broadcast),
    // not just unicast packets directly addressed to this node. Remove isForUs() check
    // to activate protection when receiving ANY VO traffic on the network.

    auto dataHeader = dynamicPtrCast<const Ieee80211DataHeader>(header);
    if (dataHeader == nullptr)
        return false;

    // Reuse EDCA classification to keep RX-trigger logic aligned with the active QoS mapping.
    return edca->classifyFrame(dataHeader) == AccessCategory::AC_VO;
}

bool V2xHcf::isEmergencyBlockingActive() const
{
    return adaptiveBlocking && emergencyPreemption && fsmController != nullptr && fsmController->isBeBlocked();
}

void V2xHcf::activateVoProtection(simtime_t duration)
{
    if (!adaptiveBlocking || fsmController == nullptr)
        return;

    ++voProtectionActivationCount;
    emit(voProtectionActivationSignal, 1L);
    const auto stateBefore = static_cast<int>(fsmController->getState());
    fsmController->onVoDemandDetected(duration);
    // #region agent log
    agentDebugLog(
        "H2",
        "src/mac/V2xHcf.cc:activateVoProtection",
        "VO protection activation updated FSM state",
        "\"duration\":" + std::to_string(duration.dbl()) +
            ",\"stateBefore\":" + std::to_string(stateBefore) +
            ",\"stateAfter\":" + std::to_string(static_cast<int>(fsmController->getState())) +
            ",\"blockingUntil\":" + std::to_string(fsmController->getBlockingUntil().dbl()) +
            ",\"activationCount\":" + std::to_string(voProtectionActivationCount) +
            ",\"simTime\":" + std::to_string(simTime().dbl()));
    // #endregion
}

void V2xHcf::dropBeWhileBlocked(Packet *packet)
{
    ++beDroppedWhileBlockedCount;
    emit(beDroppedWhileBlockedSignal, 1L);

    PacketDropDetails details;
    details.setReason(CONGESTION);
    emit(packetDroppedSignal, packet, &details);

    EV_WARN << "Dropping BE packet while emergency VO preemption is active"
            << " pkt=" << packet->getFullName()
            << " t=" << simTime()
            << endl;

    delete packet;
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

    if (ac == AccessCategory::AC_BE && isEmergencyBlockingActive()) {
        dropBeWhileBlocked(packet);
        return;
    }

    auto pendingQueue = edca->getEdcaf(ac)->getPendingQueue();
    pendingQueue->enqueuePacket(packet);

    if (ac == AccessCategory::AC_VO || (ac == AccessCategory::AC_BE && adaptiveBlocking && fsmController != nullptr && fsmController->isBeBlocked())) {
        // #region agent log
        agentDebugLog(
            ac == AccessCategory::AC_VO ? "H2" : "H4",
            "src/mac/V2xHcf.cc:processUpperFrame",
            "Upper frame queued and channel access decision state",
            "\"ac\":" + std::to_string(static_cast<int>(ac)) +
                ",\"queuePackets\":" + std::to_string(pendingQueue->getNumPackets()) +
                ",\"voQueuePackets\":" + std::to_string(edca->getEdcaf(AccessCategory::AC_VO)->getPendingQueue()->getNumPackets()) +
                ",\"beQueuePackets\":" + std::to_string(edca->getEdcaf(AccessCategory::AC_BE)->getPendingQueue()->getNumPackets()) +
                ",\"voQueueThreshold\":" + std::to_string(voQueueThreshold) +
                ",\"adaptiveBlocking\":" + std::to_string(adaptiveBlocking ? 1 : 0) +
                ",\"emergencyPreemption\":" + std::to_string(emergencyPreemption ? 1 : 0) +
                ",\"beBlocked\":" + std::to_string(fsmController != nullptr && fsmController->isBeBlocked() ? 1 : 0) +
                ",\"simTime\":" + std::to_string(simTime().dbl()));
        // #endregion
    }

    if (pendingQueue->isEmpty())
        return;

    if (adaptiveBlocking && fsmController != nullptr) {
        if (ac == AccessCategory::AC_VO && (emergencyPreemption ? hasAnyVoQueuePressure() : hasVoQueuePressure())) {
            activateVoProtection(blockDuration);
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
    if (edcaf->getAccessCategory() == AccessCategory::AC_BE && isEmergencyBlockingActive()) {
        ++beGrantSuppressedWhileBlockedCount;
        emit(beGrantSuppressedWhileBlockedSignal, 1L);

        EV_WARN << "Suppressing stale BE channel grant while emergency VO preemption is active"
                << " t=" << simTime()
                << endl;

        edcaf->releaseChannel(this);
        if (hasAnyVoQueuePressure())
            maybeRequestChannelAccess(AccessCategory::AC_VO);
        if (hasBeQueuePressure())
            scheduleBeRetry();
        return;
    }

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
        bool hasPendingVo = emergencyPreemption ? hasAnyVoQueuePressure() : hasVoQueuePressure();
        fsmController->onVoTransmissionEnd(hasPendingVo);
        if (hasPendingVo)
            activateVoProtection(blockDuration);
        else if (hasBeQueuePressure())
            scheduleBeRetry();
    }
}

void V2xHcf::processLowerFrame(Packet *packet, const Ptr<const Ieee80211MacHeader>& header)
{
    Enter_Method("processLowerFrame(%s)", packet->getName());

    // Received VO addressed to this node also extends alert mode, reducing BE contention during crash traffic.
    if (adaptiveBlocking && fsmController != nullptr && isReceivedVoDataForUs(header)) {
        activateVoProtection(blockDuration);
        if (hasBeQueuePressure())
            scheduleBeRetry();
    }

    Hcf::processLowerFrame(packet, header);
}

} // namespace veins_qos::mac
