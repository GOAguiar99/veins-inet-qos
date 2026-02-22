#pragma once

#include <omnetpp.h>

#include "veins_inet/VeinsInetApplicationBase.h"
#include "inet/common/packet/Packet.h"

namespace veins_qos::traffic {

/**
 * CrashBurstApp:
 * - At crashAt (absolute sim time), stop vehicle and start periodic VO traffic (DSCP=46).
 * - Stop VO traffic and resume driving after resumeAfter.
 */
class CrashBurstApp : public veins::VeinsInetApplicationBase
{
  protected:
    int targetNodeIndex = 0;
    simtime_t crashAt = SIMTIME_ZERO;
    simtime_t resumeAfter = SIMTIME_ZERO;
    simtime_t sendInterval = SIMTIME_ZERO;
    int payloadBytes = 0;
    std::string packetName;

    bool crashDone = false;
    bool crashActive = false;
    uint64_t txGen = 0;

  protected:
    bool startApplication() override;
    bool stopApplication() override;

    void triggerCrash();
    void resumeVehicle();
    void startCrashTraffic();
    void scheduleNext(uint64_t myGen);
    void sendOne();

    void processPacket(std::shared_ptr<inet::Packet> pk) override;
};

} // namespace veins_qos::traffic
