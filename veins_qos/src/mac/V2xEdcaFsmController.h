#pragma once

#include <omnetpp.h>

namespace veins_qos::mac {

enum class V2xState {
    LISTENING,
    BLOCKING,
    SENDING
};

class V2xEdcaFsmController : public omnetpp::cSimpleModule
{
  protected:
    V2xState state = V2xState::LISTENING;

    omnetpp::cMessage *blockTimer = nullptr;
    omnetpp::cMessage *sendingGuardTimer = nullptr;

    omnetpp::simtime_t maxContinuousBlock = -1;
    omnetpp::simtime_t sendingGuardTimeout = -1;

    omnetpp::simtime_t lastRequestedDuration = 0;
    omnetpp::simtime_t blockingStartedAt = -1;
    omnetpp::simtime_t blockingUntil = -1;

  protected:
    virtual void initialize() override;
    virtual void handleMessage(omnetpp::cMessage *msg) override;
    virtual void refreshDisplay() const override;

    omnetpp::simtime_t capBlockEnd(omnetpp::simtime_t desiredEnd) const;
    void enterListening();
    void enterBlocking(omnetpp::simtime_t desiredEnd);
    void enterSending();

  public:
    virtual ~V2xEdcaFsmController();

    void onVoDemandDetected(omnetpp::simtime_t duration);
    void onVoTransmissionStart();
    void onVoTransmissionEnd(bool hasPendingVo);

    bool isBeBlocked() const;
    bool isSending() const;
    V2xState getState() const { return state; }
};

} // namespace veins_qos::mac
