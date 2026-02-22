#pragma once

#include <omnetpp.h>

#include "veins_inet/VeinsInetApplicationBase.h"
#include "inet/common/packet/Packet.h"

namespace veins_qos::traffic {

/**
 * CrashBurstApp:
 * - At crashAt (absolute sim time), stop vehicle and send a burst with DSCP=46.
 * - Resume driving after resumeAfter.
 */
class CrashBurstApp : public veins::VeinsInetApplicationBase
{
  protected:
    int targetNodeIndex = 0;
    simtime_t crashAt = SIMTIME_ZERO;
    simtime_t resumeAfter = SIMTIME_ZERO;
    int burstPackets = 0;
    int payloadBytes = 0;
    std::string packetName;

    bool crashDone = false;

  protected:
    bool startApplication() override;
    bool stopApplication() override;

    void triggerCrash();
    void resumeVehicle();
    void sendBurst();

    void processPacket(std::shared_ptr<inet::Packet> pk) override;
};

} // namespace veins_qos::traffic
