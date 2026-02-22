#include "CritPacketSender.h"

#include "inet/common/TimeTag_m.h"
#include "inet/common/packet/chunk/ByteCountChunk.h"
#include "inet/networklayer/common/DscpTag_m.h"
#include "inet/networklayer/common/L3AddressTag_m.h"

using namespace inet;

namespace veins_qos::traffic {

Define_Module(CritPacketSender);

bool CritPacketSender::startApplication()
{
    enabled      = par("enabled").boolValue();
    sendInterval = par("sendInterval");
    payloadBytes = par("payloadBytes").intValue();
    dscp         = par("dscp").intValue();
    packetName   = par("packetName").stdstringValue();

    ++gen; // reset any previous chain (defensive)

    EV_INFO << "CritPacketSender started"
            << " idx=" << getParentModule()->getIndex()
            << " t=" << simTime()
            << " enabled=" << enabled
            << " sendInterval=" << sendInterval
            << " payloadBytes=" << payloadBytes
            << " dscp=" << dscp
            << " packetName=" << packetName
            << endl;

    if (enabled && sendInterval > SIMTIME_ZERO) {
        startLoop(sendInterval);
    }

    return true;
}

bool CritPacketSender::stopApplication()
{
    ++gen; // cancel current periodic chain
    return true;
}

void CritPacketSender::startLoop(simtime_t interval)
{
    const uint64_t myGen = ++gen;
    scheduleNext(myGen, interval);
}

void CritPacketSender::scheduleNext(uint64_t myGen, simtime_t interval)
{
    timerManager.create(
        veins::TimerSpecification([this, myGen, interval]() {
            if (myGen != gen) return;
            if (enabled) sendOne();
            scheduleNext(myGen, interval);
        }).oneshotIn(interval)
    );
}

void CritPacketSender::sendOne()
{
    auto pk = createPacket(packetName.c_str());

    // QoS marking via DSCP (mapped later by your classifier)
    pk->addTagIfAbsent<DscpReq>()->setDifferentiatedServicesCodePoint(dscp);

    // dummy payload (airtime matters)
    const auto payload = makeShared<ByteCountChunk>(B(payloadBytes));
    timestampPayload(payload); // adds CreationTimeTag inside payload
    pk->insertAtBack(payload);

    EV_INFO << "TX " << pk->getName()
            << " bytes=" << payloadBytes
            << " dscp=" << dscp
            << " t=" << simTime()
            << endl;

    sendPacket(std::move(pk));
}

void CritPacketSender::processPacket(std::shared_ptr<Packet> pk)
{
    // This sender app doesn’t need RX logic, but logging is useful.
    int rxDscp = -1;
    if (const auto dscpInd = pk->findTag<DscpInd>())
        rxDscp = dscpInd->getDifferentiatedServicesCodePoint();
    else if (const auto dscpReq = pk->findTag<DscpReq>())
        rxDscp = dscpReq->getDifferentiatedServicesCodePoint();

    simtime_t delay = SIMTIME_ZERO;
    bool hasCreationTime = false;
    for (auto& region : pk->peekData()->getAllTags<CreationTimeTag>()) {
        delay = simTime() - region.getTag()->getCreationTime();
        hasCreationTime = true;
        break;
    }

    L3Address src;
    if (const auto srcTag = pk->findTag<L3AddressInd>())
        src = srcTag->getSrcAddress();

    EV_INFO << "RX " << pk->getName()
            << " from " << src
            << " dscp=" << rxDscp
            << " delay=" << (hasCreationTime ? delay : SIMTIME_ZERO)
            << " t=" << simTime()
            << endl;
}

} // namespace veins_qos::traffic