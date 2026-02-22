#include "QosClassifier.h"
#include <cstring>
#include "inet/networklayer/common/DscpTag_m.h"

namespace veins_qos::qos {

Define_Module(QosClassifier);

void QosClassifier::initialize()
{
    crashDscp = par("crashDscp").intValue(); // default 46 in NED
}

int QosClassifier::getDscp(inet::Packet *pkt) const
{
    // Prefer explicit DSCP request tag set by upper layers/apps.
    if (const auto dscpReq = pkt->findTag<inet::DscpReq>())
        return dscpReq->getDifferentiatedServicesCodePoint();

    const inet::Ptr<const inet::PacketProtocolTag> protoTag = pkt->findTag<inet::PacketProtocolTag>();
    if (!protoTag)
        return -1;

    const inet::Protocol *proto = protoTag->getProtocol();

    if (proto == &inet::Protocol::ipv4) {
        auto ipv4 = pkt->peekAtFront<inet::Ipv4Header>();
        return ipv4->getDscp();
    }
    else if (proto == &inet::Protocol::ipv6) {
        auto ipv6 = pkt->peekAtFront<inet::Ipv6Header>();
        return ipv6->getTrafficClass() >> 2;
    }
    return -1; // non-IP
}

int QosClassifier::mapDscpToUp(int dscp) const
{
    // only what you asked: BE or VO
    return (dscp == crashDscp) ? inet::UP_VO : inet::UP_BE;
}

void QosClassifier::handleMessage(omnetpp::cMessage *msg)
{
    auto *pkt = omnetpp::check_and_cast<inet::Packet *>(msg);

    const int dscp = getDscp(pkt);
    const int up   = mapDscpToUp(dscp);

    pkt->addTagIfAbsent<inet::UserPriorityReq>()->setUserPriority(up);

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
