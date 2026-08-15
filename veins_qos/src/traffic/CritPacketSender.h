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
 * - Sends one packet after each sendInterval draw (if enabled)
 * - Adds DscpReq tag (dscp param)
 * - Appends dummy payload with payloadBytes
 *
 * sendInterval is volatile: each packet draws a new value so
 * exponential(mean) in omnetpp.ini is a Poisson process, not a
 * per-node CBR period frozen at initialize.
 *
 * This module does NOT implement any crash logic. It just sends.
 */
class CritPacketSender : public veins::VeinsInetApplicationBase
{
  protected:
    bool enabled = true;
    int payloadBytes = 0;
    int dscp = 0;
    int crashNodeIndex = 0;
    std::string packetName;
    inet::L3Address selfAddress;
    inet::L3Address crashNodeAddress;
    simtime_t voDedupWindow = SIMTIME_ZERO;

    uint64_t gen = 0; // cancels old timer chain when stopping
    std::map<std::pair<std::string, int>, simtime_t> voDedupSeen;

  protected:
    bool startApplication() override;
    bool stopApplication() override;

    void startLoop();
    void scheduleNext(uint64_t myGen);
    omnetpp::simtime_t drawSendInterval() const;
    inet::L3Address resolveCrashNodeAddress() const;

    void sendOne();

    void processPacket(std::shared_ptr<inet::Packet> pk) override;
};

} // namespace veins_qos::traffic
