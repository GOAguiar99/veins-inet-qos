#include "CrashBurstApp.h"

#include "inet/common/packet/chunk/ByteCountChunk.h"
#include "inet/networklayer/common/DscpTag_m.h"

using namespace inet;

namespace veins_qos::traffic {

Define_Module(CrashBurstApp);

namespace {
constexpr int kDscpVo = 46;
const simsignal_t kVoTxPacketCountSignal = cComponent::registerSignal("voTxPacketCount");
} // namespace

bool CrashBurstApp::startApplication()
{
    targetNodeIndex = par("targetNodeIndex").intValue();
    crashAt = par("crashAt");
    resumeAfter = par("resumeAfter");
    sendInterval = par("sendInterval");
    payloadBytes = par("payloadBytes").intValue();
    packetName = par("packetName").stdstringValue();
    const int myIndex = getParentModule()->getIndex();

    EV_INFO << "CrashBurstApp started"
            << " idx=" << myIndex
            << " t=" << simTime()
            << " targetNodeIndex=" << targetNodeIndex
            << " crashAt=" << crashAt
            << " resumeAfter=" << resumeAfter
            << " sendInterval=" << sendInterval
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
    ++txGen; // cancel periodic VO traffic chain
    crashActive = false;
    return true;
}

void CrashBurstApp::triggerCrash()
{
    if (crashDone) return;
    crashDone = true;

    EV_WARN << "CRASH TRIGGER"
            << " t=" << simTime()
            << " starting periodic VO traffic"
            << " dscp=" << kDscpVo
            << endl;

    getParentModule()->getDisplayString().setTagArg("i", 1, "red");

    if (traciVehicle) {
        traciVehicle->setSpeed(0);
    }

    startCrashTraffic();

    if (resumeAfter > SIMTIME_ZERO) {
        timerManager.create(
            veins::TimerSpecification([this]() { resumeVehicle(); })
                .oneshotIn(resumeAfter)
        );
    }
}

void CrashBurstApp::startCrashTraffic()
{
    if (sendInterval <= SIMTIME_ZERO) {
        EV_WARN << "VO traffic disabled because sendInterval <= 0"
                << " sendInterval=" << sendInterval
                << endl;
        return;
    }

    crashActive = true;
    const uint64_t myGen = ++txGen;

    // Start immediately at crash time, then keep periodic cadence.
    sendOne();
    scheduleNext(myGen);
}

void CrashBurstApp::scheduleNext(uint64_t myGen)
{
    timerManager.create(
        veins::TimerSpecification([this, myGen]() {
            if (myGen != txGen || !crashActive) return;
            sendOne();
            scheduleNext(myGen);
        }).oneshotIn(sendInterval)
    );
}

void CrashBurstApp::sendOne()
{
    auto pk = createPacket(packetName.c_str());

    pk->addTagIfAbsent<DscpReq>()->setDifferentiatedServicesCodePoint(kDscpVo);

    const auto payload = makeShared<ByteCountChunk>(B(payloadBytes));
    timestampPayload(payload);
    pk->insertAtBack(payload);

    EV_INFO << "TX " << pk->getName()
            << " bytes=" << payloadBytes
            << " dscp=" << kDscpVo
            << " t=" << simTime()
            << endl;

    sendPacket(std::move(pk));
    emit(kVoTxPacketCountSignal, 1L);
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

    crashActive = false;
    ++txGen; // stop periodic VO traffic

    if (traciVehicle) {
        traciVehicle->setSpeed(-1);
    }
}

} // namespace veins_qos::traffic
