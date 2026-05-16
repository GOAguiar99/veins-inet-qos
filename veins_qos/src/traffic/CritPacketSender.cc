#include "CritPacketSender.h"

#include <fstream>
#include <string>

#include "inet/common/SequenceNumberTag_m.h"
#include "inet/common/TimeTag_m.h"
#include "inet/common/packet/chunk/ByteCountChunk.h"
#include "inet/networklayer/common/DscpTag_m.h"
#include "inet/networklayer/common/L3AddressTag_m.h"
#include "inet/networklayer/common/L3AddressResolver.h"

using namespace inet;

namespace veins_qos::traffic {

Define_Module(CritPacketSender);

namespace {
constexpr int kDscpBe = 0;
constexpr int kDscpVo = 46;

const simsignal_t kBeTxPacketCountSignal = cComponent::registerSignal("beTxPacketCount");
const simsignal_t kBeRxPacketCountSignal = cComponent::registerSignal("beRxPacketCount");
const simsignal_t kVoRxPacketCountSignal = cComponent::registerSignal("voRxPacketCount");
const simsignal_t kBeE2eDelaySignal = cComponent::registerSignal("beE2eDelay");
const simsignal_t kVoE2eDelaySignal = cComponent::registerSignal("voE2eDelay");

int parseCrashSequenceFromName(const char *name)
{
    const std::string text = name == nullptr ? "" : name;
    const auto repeatMarker = text.rfind("_r");
    if (repeatMarker == std::string::npos || repeatMarker == 0)
        return -1;

    const auto sequenceStart = text.rfind('_', repeatMarker - 1);
    if (sequenceStart == std::string::npos || sequenceStart + 1 >= repeatMarker)
        return -1;

    try {
        const auto sequenceText = text.substr(sequenceStart + 1, repeatMarker - sequenceStart - 1);
        std::size_t consumed = 0;
        const int sequence = std::stoi(sequenceText, &consumed);
        return consumed == sequenceText.size() ? sequence : -1;
    }
    catch (...) {
        return -1;
    }
}

void agentDebugLog(const char *hypothesisId, const char *location, const char *message, const std::string& data)
{
    std::ofstream out("/home/goaguiar/master/master_veins/.cursor/debug-9574a1.log", std::ios::app);
    out << "{\"sessionId\":\"9574a1\",\"runId\":\"pre-fix\",\"hypothesisId\":\"" << hypothesisId
        << "\",\"location\":\"" << location << "\",\"message\":\"" << message
        << "\",\"data\":{" << data << "},\"timestamp\":" << static_cast<long long>(omnetpp::simTime().dbl() * 1000) << "}\n";
}
} // namespace

bool CritPacketSender::startApplication()
{
    enabled      = par("enabled").boolValue();
    sendInterval = par("sendInterval");
    payloadBytes = par("payloadBytes").intValue();
    dscp         = par("dscp").intValue();
    packetName   = par("packetName").stdstringValue();
    voDedupWindow = par("voDedupWindow");
    selfAddress  = L3AddressResolver().addressOf(getParentModule(), "wlan0");

    ++gen; // reset any previous chain (defensive)

    EV_INFO << "CritPacketSender started"
            << " idx=" << getParentModule()->getIndex()
            << " t=" << simTime()
            << " enabled=" << enabled
            << " sendInterval=" << sendInterval
            << " payloadBytes=" << payloadBytes
            << " dscp=" << dscp
            << " voDedupWindow=" << voDedupWindow
            << " selfAddress=" << selfAddress
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
    voDedupSeen.clear();
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
    emit(kBeTxPacketCountSignal, 1L);
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

    int seq = -1;
    if (const auto seqInd = pk->findTag<SequenceNumberInd>())
        seq = seqInd->getSequenceNumber();
    else if (const auto seqReq = pk->findTag<SequenceNumberReq>())
        seq = seqReq->getSequenceNumber();
    else if (rxDscp == kDscpVo)
        seq = parseCrashSequenceFromName(pk->getName());

    const bool isSelfOrigin = !src.isUnspecified() && src == selfAddress;
    const int nodeIndex = getParentModule()->getIndex();
    const bool countedForKpi = !isSelfOrigin && (rxDscp == kDscpBe || rxDscp == kDscpVo);

    if (nodeIndex == 1 && (rxDscp == kDscpBe || rxDscp == kDscpVo)) {
        // #region agent log
        agentDebugLog(
            rxDscp == kDscpVo ? "H3,H5" : "H5",
            "src/traffic/CritPacketSender.cc:processPacket",
            "Application receive delay and KPI eligibility sample",
            "\"nodeIndex\":" + std::to_string(nodeIndex) +
                ",\"rxDscp\":" + std::to_string(rxDscp) +
                ",\"sequence\":" + std::to_string(seq) +
                ",\"delay\":" + std::to_string(delay.dbl()) +
                ",\"hasCreationTime\":" + std::to_string(hasCreationTime ? 1 : 0) +
                ",\"selfOrigin\":" + std::to_string(isSelfOrigin ? 1 : 0) +
                ",\"countedForKpi\":" + std::to_string(countedForKpi ? 1 : 0) +
                ",\"receiveTime\":" + std::to_string(simTime().dbl()));
        // #endregion
    }

    EV_INFO << "RX " << pk->getName()
            << " from " << src
            << " dscp=" << rxDscp
            << " seq=" << seq
            << " delay=" << (hasCreationTime ? delay : SIMTIME_ZERO)
            << " selfOrigin=" << (isSelfOrigin ? "yes" : "no")
            << " t=" << simTime()
            << endl;

    // Multicast is also delivered locally; exclude looped-back self traffic from e2e KPIs.
    if (isSelfOrigin)
        return;

    if (rxDscp == kDscpBe) {
        emit(kBeRxPacketCountSignal, 1L);
        if (hasCreationTime) emit(kBeE2eDelaySignal, delay);
    }
    else if (rxDscp == kDscpVo) {
        if (voDedupWindow > SIMTIME_ZERO && seq >= 0 && !src.isUnspecified()) {
            auto key = std::make_pair(src.str(), seq);
            auto it = voDedupSeen.find(key);
            if (it != voDedupSeen.end()) {
                EV_DEBUG << "Skipping duplicate VO packet src=" << src << " seq=" << seq << endl;
                return;
            }
            voDedupSeen[key] = simTime();
        }

        emit(kVoRxPacketCountSignal, 1L);
        if (hasCreationTime) emit(kVoE2eDelaySignal, delay);
    }
}

} // namespace veins_qos::traffic
