#include "veins_inet/VeinsInetCritTrafficApp.h"

#include "inet/linklayer/common/UserPriorityTag_m.h"   // UserPriorityReq
#include "inet/networklayer/common/L3AddressTag_m.h"

using namespace inet;

namespace veins_qos::veins_inet {

Define_Module(VeinsInetCritTrafficApp);

bool VeinsInetCritTrafficApp::startApplication()
{
    crashEnabled = par("crashEnabled").boolValue();
    crashTime = par("crashTime");
    crashDuration = par("crashDuration");

    sendInterval = par("sendInterval");
    crashSendInterval = par("crashSendInterval");
    payloadBytes = par("payloadBytes").intValue();

    crashActive = false;
    startLoop(sendInterval);

    if (crashEnabled && crashTime >= SIMTIME_ZERO) {
        timerManager.create(veins::TimerSpecification([this]() { triggerCrash(); })
                                .oneshotAt(crashTime));
    }
    return true;
}

bool VeinsInetCritTrafficApp::stopApplication()
{
    ++gen; // stop current periodic chain
    return true;
}

void VeinsInetCritTrafficApp::startLoop(simtime_t interval)
{
    const uint64_t myGen = ++gen; // bump once per mode switch
    scheduleNext(myGen, interval);
}

void VeinsInetCritTrafficApp::scheduleNext(uint64_t myGen, simtime_t interval)
{
    timerManager.create(veins::TimerSpecification([this, myGen, interval]() {
                            if (myGen != gen) return;
                            sendOne();
                            scheduleNext(myGen, interval);
                        }).oneshotIn(interval));
}

void VeinsInetCritTrafficApp::sendOne()
{
    auto pk = createPacket(crashActive ? "crash_vo" : "be");

    // Tag packet for EDCA (bypasses DSCP mapping ambiguity)
    pk->addTagIfAbsent<UserPriorityReq>()
        ->setUserPriority(crashActive ? UP_VO : UP_BE);

    // Dummy payload (size matters for airtime)
    const auto payload = makeShared<BytesChunk>(B(payloadBytes));
    pk->insertAtBack(payload);

    sendPacket(std::move(pk));
}

void VeinsInetCritTrafficApp::triggerCrash()
{
    crashActive = true;
    startLoop(crashSendInterval);

    if (crashDuration > SIMTIME_ZERO) {
        timerManager.create(veins::TimerSpecification([this]() { clearCrash(); })
                                .oneshotIn(crashDuration));
    }
}

void VeinsInetCritTrafficApp::clearCrash()
{
    crashActive = false;
    startLoop(sendInterval);
}

void VeinsInetCritTrafficApp::processPacket(std::shared_ptr<Packet> pk)
{
    // optional: keep empty if you don't care about RX
    // auto src = pk->getTag<L3AddressInd>()->getSrcAddress();
    // EV_INFO << "RX " << pk->getName() << " from " << src << "\n";
}

} // namespace veins_qos::veins_inet