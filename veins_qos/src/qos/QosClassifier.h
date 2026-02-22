#pragma once

#include <omnetpp.h>

#include "inet/common/INETDefs.h"
#include "inet/common/Protocol.h"
#include "inet/common/packet/Packet.h"
#include "inet/common/ProtocolTag_m.h"
#include "inet/common/IProtocolRegistrationListener.h"

#include "inet/linklayer/common/UserPriority.h"
#include "inet/linklayer/common/UserPriorityTag_m.h"

#include "inet/networklayer/common/DscpTag_m.h"
#include "inet/networklayer/ipv4/Ipv4Header_m.h"
#include "inet/networklayer/ipv6/Ipv6Header.h"

namespace veins_qos::qos {

class QosClassifier : public omnetpp::cSimpleModule,
                      public inet::DefaultProtocolRegistrationListener
{
  protected:
    int crashDscp = 46;
    int defaultUp = inet::UP_BE;

  protected:
    void initialize() override;
    void handleMessage(omnetpp::cMessage *msg) override;

    int getDscp(inet::Packet *pkt) const;
    int mapDscpToUp(int dscp) const;

    // protocol registration plumbing
    void handleRegisterService(const inet::Protocol& protocol,
                               omnetpp::cGate *g,
                               inet::ServicePrimitive prim) override;

    void handleRegisterProtocol(const inet::Protocol& protocol,
                                omnetpp::cGate *g,
                                inet::ServicePrimitive prim) override;
};

} // namespace veins_qos::qos