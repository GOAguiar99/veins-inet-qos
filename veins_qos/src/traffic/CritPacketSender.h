#pragma once

#include <map>
#include <string>

#include <omnetpp.h>

#include "veins_inet/VeinsInetApplicationBase.h"
#include "inet/common/packet/Packet.h"
#include "inet/networklayer/common/L3Address.h"

namespace veins_qos::traffic {

/**
 * Periodic packet generator.
 * - Sends one packet every sendInterval (if enabled)
 * - Adds DscpReq tag (dscp param)
 * - Appends dummy payload with payloadBytes
 *
 * This module does NOT implement any crash logic. It just sends.
 */
class CritPacketSender : public veins::VeinsInetApplicationBase
{
  protected:
    // params
    bool enabled = true;
    simtime_t sendInterval = SIMTIME_ZERO;
    int payloadBytes = 0;
    int dscp = 0;
    std::string packetName;
    inet::L3Address selfAddress;
    simtime_t voDedupWindow = SIMTIME_ZERO;

    // state
    uint64_t gen = 0; // cancels old timer chain when stopping
    std::map<std::pair<std::string, int>, simtime_t> voDedupSeen;

  protected:
    bool startApplication() override;
    bool stopApplication() override;

    void startLoop(simtime_t interval);
    void scheduleNext(uint64_t myGen, simtime_t interval);

    void sendOne();

    void processPacket(std::shared_ptr<inet::Packet> pk) override;
};

} // namespace veins_qos::traffic
