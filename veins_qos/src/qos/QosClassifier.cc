#include "QosClassifier.h"

#include <cstring>

#include "inet/common/ProtocolTag_m.h"
#include "inet/networklayer/common/DscpTag_m.h"
#include "inet/linklayer/common/UserPriorityTag_m.h"

namespace veins_qos::qos {

Define_Module(QosClassifier);

void QosClassifier::initialize()
{
    crashDscp = par("crashDscp").intValue(); // e.g., 46
    defaultUp = par("defaultUp").intValue(); // e.g., inet::UP_BE (0)
}

// Robust DSCP extraction for classification:
// 1) Prefer indication (post-processing / already-decoded)
// 2) Then request (set by app / upper layers)
// 3) Optional: parse IPv4/IPv6 only if we can safely identify it at the front
int QosClassifier::getDscp(inet::Packet *pkt) const
{
    if (pkt == nullptr) return -1;

    // Most reliable after network-layer processing
    if (const auto dscpInd = pkt->findTag<inet::DscpInd>())
        return dscpInd->getDifferentiatedServicesCodePoint();

    // Most reliable before network-layer encapsulation (your app sets this)
    if (const auto dscpReq = pkt->findTag<inet::DscpReq>())
        return dscpReq->getDifferentiatedServicesCodePoint();

    // Fallback: only attempt header parsing if the protocol tag says
    // the network header is at the front of the packet.
    const auto protoTag = pkt->findTag<inet::PacketProtocolTag>();
    if (!protoTag)
        return -1;

    const inet::Protocol *proto = protoTag->getProtocol();

    try {
        if (proto == &inet::Protocol::ipv4) {
            const auto ipv4 = pkt->peekAtFront<inet::Ipv4Header>();
            return ipv4->getDscp();
        }
        else if (proto == &inet::Protocol::ipv6) {
            const auto ipv6 = pkt->peekAtFront<inet::Ipv6Header>();
            // DSCP = top 6 bits of Traffic Class
            return static_cast<int>((ipv6->getTrafficClass() & 0xFF) >> 2);
        }
    }
    catch (const omnetpp::cRuntimeError&) {
        // Not at front / not present; treat as unknown DSCP
        return -1;
    }

    return -1;
}

int QosClassifier::mapDscpToUp(int dscp) const
{
    // Minimal two-class policy:
    // - crash DSCP -> VO
    // - unknown / everything else -> defaultUp (typically BE)
    if (dscp == crashDscp)
        return inet::UP_VO;

    return defaultUp;
}

void QosClassifier::handleMessage(omnetpp::cMessage *msg)
{
    auto *pkt = omnetpp::check_and_cast<inet::Packet *>(msg);

    const int dscp = getDscp(pkt);
    const int up   = mapDscpToUp(dscp);

    // Only add/override if needed; avoid repeatedly rewriting tags
    auto upTag = pkt->addTagIfAbsent<inet::UserPriorityReq>();
    if (upTag->getUserPriority() != up)
        upTag->setUserPriority(up);

    // (Optional) also annotate the DSCP we used, if none existed
    // This helps debugging downstream without changing already-present DSCP tags.
    if (dscp >= 0 && !pkt->hasTag<inet::DscpReq>() && !pkt->hasTag<inet::DscpInd>())
        pkt->addTagIfAbsent<inet::DscpReq>()->setDifferentiatedServicesCodePoint(dscp);

    send(pkt, "out");
}

// registration plumbing
void QosClassifier::handleRegisterService(const inet::Protocol& protocol, omnetpp::cGate *g, inet::ServicePrimitive prim)
{
    Enter_Method("handleRegisterService");
    if (!strcmp("in", g->getName()))
        registerService(protocol, gate("out"), prim);
    else if (!strcmp("out", g->getName()))
        registerService(protocol, gate("in"), prim);
    else
        throw omnetpp::cRuntimeError("Unknown gate: %s", g->getName());
}

void QosClassifier::handleRegisterProtocol(const inet::Protocol& protocol, omnetpp::cGate *g, inet::ServicePrimitive prim)
{
    Enter_Method("handleRegisterProtocol");
    if (!strcmp("in", g->getName()))
        registerProtocol(protocol, gate("out"), prim);
    else
        throw omnetpp::cRuntimeError("Unknown gate: %s", g->getName());
}

} // namespace veins_qos::qos