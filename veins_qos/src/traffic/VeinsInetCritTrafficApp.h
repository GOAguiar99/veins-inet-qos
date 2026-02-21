#pragma once

#include "veins_inet/VeinsInetApplicationBase.h"
#include "inet/common/packet/Packet.h"

namespace veins_qos::traffic {

class VeinsInetCritTrafficApp : public veins::VeinsInetApplicationBase
{
  protected:
    // params
    bool crashEnabled = false;
    simtime_t crashTime = SIMTIME_ZERO;
    simtime_t crashDuration = SIMTIME_ZERO;
    simtime_t sendInterval = SIMTIME_ZERO;
    simtime_t crashSendInterval = SIMTIME_ZERO;
    int payloadBytes = 200;

    // state
    bool crashActive = false;
    uint64_t gen = 0;

    static constexpr int UP_BE = 0; // AC_BE
    static constexpr int UP_VO = 6; // AC_VO

  protected:
    virtual bool startApplication() override;
    virtual bool stopApplication() override;
    virtual void processPacket(std::shared_ptr<inet::Packet> pk) override;

  protected:
    void startLoop(inet::simtime_t interval);
    void scheduleNext(uint64_t myGen, inet::simtime_t interval);
    void sendOne();

    void triggerCrash();
    void clearCrash();
};

} // namespace veins_qos::traffic
