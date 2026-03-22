#include "mac/V2xEdcaFsmController.h"

namespace veins_qos::mac {

using namespace omnetpp;

Define_Module(V2xEdcaFsmController);

V2xEdcaFsmController::~V2xEdcaFsmController()
{
    cancelAndDelete(blockTimer);
    cancelAndDelete(sendingGuardTimer);
}

void V2xEdcaFsmController::initialize()
{
    maxContinuousBlock = par("maxContinuousBlock");
    sendingGuardTimeout = par("sendingGuardTimeout");

    blockTimer = new omnetpp::cMessage("v2xBlockTimer");
    sendingGuardTimer = new omnetpp::cMessage("v2xSendingGuardTimer");

    enterListening();
}

simtime_t V2xEdcaFsmController::capBlockEnd(simtime_t desiredEnd) const
{
    if (maxContinuousBlock > SIMTIME_ZERO && blockingStartedAt >= SIMTIME_ZERO) {
        auto capEnd = blockingStartedAt + maxContinuousBlock;
        if (desiredEnd > capEnd)
            desiredEnd = capEnd;
    }
    return desiredEnd;
}

void V2xEdcaFsmController::enterListening()
{
    state = V2xState::LISTENING;
    blockingStartedAt = -1;
    blockingUntil = -1;
    if (blockTimer->isScheduled())
        cancelEvent(blockTimer);
    if (sendingGuardTimer->isScheduled())
        cancelEvent(sendingGuardTimer);
    refreshDisplay();
}

void V2xEdcaFsmController::enterBlocking(simtime_t desiredEnd)
{
    if (blockingStartedAt < SIMTIME_ZERO)
        blockingStartedAt = simTime();

    blockingUntil = capBlockEnd(desiredEnd);
    if (blockingUntil <= simTime()) {
        enterListening();
        return;
    }

    state = V2xState::BLOCKING;
    if (blockTimer->isScheduled())
        rescheduleAt(blockingUntil, blockTimer);
    else
        scheduleAt(blockingUntil, blockTimer);

    if (sendingGuardTimer->isScheduled())
        cancelEvent(sendingGuardTimer);

    refreshDisplay();
}

void V2xEdcaFsmController::enterSending()
{
    state = V2xState::SENDING;
    if (blockTimer->isScheduled())
        cancelEvent(blockTimer);

    if (sendingGuardTimeout > SIMTIME_ZERO) {
        if (sendingGuardTimer->isScheduled())
            rescheduleAt(simTime() + sendingGuardTimeout, sendingGuardTimer);
        else
            scheduleAt(simTime() + sendingGuardTimeout, sendingGuardTimer);
    }

    refreshDisplay();
}

void V2xEdcaFsmController::handleMessage(omnetpp::cMessage *msg)
{
    if (msg == blockTimer) {
        if (state == V2xState::BLOCKING)
            enterListening();
    }
    else if (msg == sendingGuardTimer) {
        if (state == V2xState::SENDING) {
            if (blockingUntil > simTime())
                enterBlocking(blockingUntil);
            else
                enterListening();
        }
    }
    else
        throw omnetpp::cRuntimeError("Unknown message received");
}

void V2xEdcaFsmController::onVoDemandDetected(simtime_t duration)
{
    Enter_Method("onVoDemandDetected");

    if (duration <= SIMTIME_ZERO)
        duration = sendingGuardTimeout;
    if (duration <= SIMTIME_ZERO)
        return;

    lastRequestedDuration = duration;

    if (state == V2xState::LISTENING || blockingStartedAt < SIMTIME_ZERO)
        blockingStartedAt = simTime();

    simtime_t desiredEnd = simTime() + duration;
    if (blockingUntil < SIMTIME_ZERO || desiredEnd > blockingUntil)
        blockingUntil = desiredEnd;
    blockingUntil = capBlockEnd(blockingUntil);

    if (state == V2xState::LISTENING || state == V2xState::BLOCKING)
        enterBlocking(blockingUntil);
    else
        refreshDisplay();
}

void V2xEdcaFsmController::onVoTransmissionStart()
{
    Enter_Method("onVoTransmissionStart");

    if (state == V2xState::SENDING)
        return;

    if (blockingStartedAt < SIMTIME_ZERO)
        blockingStartedAt = simTime();

    if (blockingUntil <= simTime()) {
        simtime_t fallbackDuration = lastRequestedDuration;
        if (fallbackDuration <= SIMTIME_ZERO)
            fallbackDuration = sendingGuardTimeout;
        if (fallbackDuration > SIMTIME_ZERO)
            blockingUntil = capBlockEnd(simTime() + fallbackDuration);
        else
            blockingUntil = simTime();
    }

    enterSending();
}

void V2xEdcaFsmController::onVoTransmissionEnd(bool hasPendingVo)
{
    Enter_Method("onVoTransmissionEnd");

    if (sendingGuardTimer->isScheduled())
        cancelEvent(sendingGuardTimer);

    if (!hasPendingVo) {
        if (blockingUntil > simTime())
            enterBlocking(blockingUntil);
        else
            enterListening();
        return;
    }

    simtime_t duration = lastRequestedDuration;
    if (duration <= SIMTIME_ZERO)
        duration = sendingGuardTimeout;
    if (duration <= SIMTIME_ZERO)
        duration = SimTime(0.001);

    if (blockingStartedAt < SIMTIME_ZERO)
        blockingStartedAt = simTime();

    simtime_t desiredEnd = simTime() + duration;
    if (desiredEnd > blockingUntil)
        blockingUntil = desiredEnd;

    enterBlocking(blockingUntil);
}

bool V2xEdcaFsmController::isBeBlocked() const
{
    return state == V2xState::BLOCKING || state == V2xState::SENDING;
}

bool V2xEdcaFsmController::isSending() const
{
    return state == V2xState::SENDING;
}

void V2xEdcaFsmController::refreshDisplay() const
{
    const char *stateStr = "EDCA: LISTENING";
    if (state == V2xState::BLOCKING)
        stateStr = "EDCA: BLOCKING";
    else if (state == V2xState::SENDING)
        stateStr = "EDCA: SENDING";

    getDisplayString().setTagArg("t", 0, stateStr);
}

} // namespace veins_qos::mac
