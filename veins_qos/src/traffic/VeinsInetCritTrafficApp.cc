#include "./VeinsInetCritTrafficApp.h"

#include "inet/networklayer/common/L3AddressTag_m.h"
#include "inet/networklayer/common/DscpTag_m.h"
#include "inet/common/packet/chunk/ByteCountChunk.h"
#include "inet/common/TimeTag_m.h"
#include <cstring>

using namespace inet;

namespace veins_qos::traffic {

Define_Module(VeinsInetCritTrafficApp);

bool VeinsInetCritTrafficApp::startApplication()
{
    crashEnabled = par("crashEnabled").boolValue();
    crashTime = par("crashTime");
    crashDuration = par("crashDuration");

    sendInterval = par("sendInterval");
    crashSendInterval = par("crashSendInterval");
    voBurstPackets = par("voBurstPackets").intValue();
    payloadBytes = par("payloadBytes").intValue();

    crashActive = false;
    startLoop(sendInterval);

    if (crashEnabled && crashTime >= SIMTIME_ZERO) {
        timerManager.create(veins::TimerSpecification([this]() { triggerCrash(); })
                                .oneshotIn(crashTime));
    }
    EV_INFO << "VeinsInetCritTrafficApp started: sendInterval=" << sendInterval
            << " crashSendInterval=" << crashSendInterval
            << " voBurstPackets=" << voBurstPackets
            << " payloadBytes=" << payloadBytes
            << " crashEnabled=" << crashEnabled << endl;
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
    static omnetpp::simsignal_t txVoPkSignal = registerSignal("txVoPk");
    static omnetpp::simsignal_t txBePkSignal = registerSignal("txBePk");

    auto pk = createPacket(crashActive ? "crash_vo" : "be");

    // Let the QoS classifier decide UP based on DSCP (VO for crash, BE otherwise)
    pk->addTagIfAbsent<DscpReq>()
        ->setDifferentiatedServicesCodePoint(crashActive ? 46 : 0);

    // Dummy payload (size matters for airtime)
    const auto payload = makeShared<ByteCountChunk>(B(payloadBytes));
    timestampPayload(payload);
    pk->insertAtBack(payload);

    EV_INFO << "TX " << pk->getName()
            << " bytes=" << payloadBytes
            << " dscp=" << (crashActive ? 46 : 0)
            << " t=" << simTime() << endl;

    emit(crashActive ? txVoPkSignal : txBePkSignal, 1L);
    sendPacket(std::move(pk));
}

void VeinsInetCritTrafficApp::triggerCrash()
{
    crashActive = true;

    for (int i = 0; i < voBurstPackets; ++i)
        sendOne();

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
    static omnetpp::simsignal_t voDelaySignal = registerSignal("voDelay");
    static omnetpp::simsignal_t beDelaySignal = registerSignal("beDelay");
    static omnetpp::simsignal_t rxVoPkSignal = registerSignal("rxVoPk");
    static omnetpp::simsignal_t rxBePkSignal = registerSignal("rxBePk");

    simtime_t delay = SIMTIME_ZERO;
    bool hasCreationTime = false;
    for (auto& region : pk->peekData()->getAllTags<CreationTimeTag>()) {
        delay = simTime() - region.getTag()->getCreationTime();
        hasCreationTime = true;
        break;
    }

    int dscp = -1;
    if (const auto dscpInd = pk->findTag<DscpInd>())
        dscp = dscpInd->getDifferentiatedServicesCodePoint();
    else if (const auto dscpReq = pk->findTag<DscpReq>())
        dscp = dscpReq->getDifferentiatedServicesCodePoint();

    const bool isVo = (dscp == 46);
    emit(isVo ? rxVoPkSignal : rxBePkSignal, 1L);
    if (hasCreationTime)
        emit(isVo ? voDelaySignal : beDelaySignal, delay);

    auto src = pk->getTag<L3AddressInd>()->getSrcAddress();
    EV_INFO << "RX " << pk->getName() << " from " << src
            << " dscp=" << dscp
            << " delay=" << delay << " t=" << simTime() << endl;
}

} // namespace veins_qos::traffic
