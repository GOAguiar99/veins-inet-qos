#pragma once

#include "inet/common/INETDefs.h"
#include "inet/common/Protocol.h"
#include "inet/common/packet/Packet.h"
#include "inet/common/ProtocolTag_m.h"
#include "inet/linklayer/common/UserPriority.h"
#include "inet/linklayer/common/UserPriorityTag_m.h"
#include "inet/networklayer/ipv4/Ipv4Header_m.h"
#include "inet/networklayer/ipv6/Ipv6Header.h"
#include "inet/common/IProtocolRegistrationListener.h"

namespace veins_qos::qos {

class QosClassifier : public omnetpp::cSimpleModule, public inet::DefaultProtocolRegistrationListener
{
  protected:
    // optional param if you want to change later
    int crashDscp = 46;

  protected:
    virtual void initialize() override;
    virtual void handleMessage(omnetpp::cMessage *msg) override;

    int getDscp(inet::Packet *pkt) const;
    int mapDscpToUp(int dscp) const;

    // plumbing (same pattern you used)
    virtual void handleRegisterService(const inet::Protocol& protocol, omnetpp::cGate *g, inet::ServicePrimitive prim) override;
    virtual void handleRegisterProtocol(const inet::Protocol& protocol, omnetpp::cGate *g, inet::ServicePrimitive prim) override;
};

} // namespace veins_qos::qos
