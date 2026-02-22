#include "CrashBurstApp.h"

#include "inet/common/packet/chunk/ByteCountChunk.h"
#include "inet/networklayer/common/DscpTag_m.h"

using namespace inet;

namespace veins_qos::traffic {

Define_Module(CrashBurstApp);

bool CrashBurstApp::startApplication()
{
    targetNodeIndex = par("targetNodeIndex").intValue();
    crashAt = par("crashAt");
    resumeAfter = par("resumeAfter");
    burstPackets = par("burstPackets").intValue();
    payloadBytes = par("payloadBytes").intValue();
    packetName = par("packetName").stdstringValue();
    const int myIndex = getParentModule()->getIndex();

    EV_INFO << "CrashBurstApp started"
            << " idx=" << myIndex
            << " t=" << simTime()
            << " targetNodeIndex=" << targetNodeIndex
            << " crashAt=" << crashAt
            << " resumeAfter=" << resumeAfter
            << " burstPackets=" << burstPackets
            << " payloadBytes=" << payloadBytes
            << endl;

    if (targetNodeIndex >= 0 && myIndex != targetNodeIndex) {
        EV_INFO << "CrashBurstApp inactive on this node"
                << " idx=" << myIndex
                << " targetNodeIndex=" << targetNodeIndex
                << endl;
        return true;
    }

    if (crashAt >= SIMTIME_ZERO) {
        if (crashAt <= simTime()) {
            EV_WARN << "crashAt already passed at startup"
                    << " now=" << simTime()
                    << " crashAt=" << crashAt
                    << " triggering immediately"
                    << endl;
            timerManager.create(
                veins::TimerSpecification([this]() { triggerCrash(); })
                    .oneshotIn(SIMTIME_ZERO)
            );
        }
        else {
            timerManager.create(
                veins::TimerSpecification([this]() { triggerCrash(); })
                    .oneshotAt(crashAt)
            );
        }
    }

    return true;
}

bool CrashBurstApp::stopApplication()
{
    return true;
}

void CrashBurstApp::triggerCrash()
{
    if (crashDone) return;
    crashDone = true;

    EV_WARN << "CRASH TRIGGER"
            << " t=" << simTime()
            << " sending burstPackets=" << burstPackets
            << " dscp=46"
            << endl;

    getParentModule()->getDisplayString().setTagArg("i", 1, "red");

    if (traciVehicle) {
        traciVehicle->setSpeed(0);
    }

    sendBurst();

    if (resumeAfter > SIMTIME_ZERO) {
        timerManager.create(
            veins::TimerSpecification([this]() { resumeVehicle(); })
                .oneshotIn(resumeAfter)
        );
    }
}

void CrashBurstApp::sendBurst()
{
    for (int i = 0; i < burstPackets; ++i) {
        auto pk = createPacket(packetName.c_str());

        pk->addTagIfAbsent<DscpReq>()->setDifferentiatedServicesCodePoint(46);

        const auto payload = makeShared<ByteCountChunk>(B(payloadBytes));
        timestampPayload(payload);
        pk->insertAtBack(payload);

        EV_INFO << "TX " << pk->getName()
                << " seq=" << i
                << " bytes=" << payloadBytes
                << " dscp=46"
                << " t=" << simTime()
                << endl;

        sendPacket(std::move(pk));
    }
}

void CrashBurstApp::processPacket(std::shared_ptr<Packet> pk)
{
    EV_DEBUG << "RX " << pk->getName() << " t=" << simTime() << endl;
}

void CrashBurstApp::resumeVehicle()
{
    EV_WARN << "CRASH RESUME"
            << " t=" << simTime()
            << endl;

    if (traciVehicle) {
        traciVehicle->setSpeed(-1);
    }
}

} // namespace veins_qos::traffic
