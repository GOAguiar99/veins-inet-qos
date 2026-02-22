package veins_qos.traffic;

import veins_qos.veins_inet.VeinsInetApplicationBase;

simple CrashBurstApp extends VeinsInetApplicationBase
{
    parameters:
        @class(veins_qos::traffic::CrashBurstApp);
        // only this OMNeT++ node index triggers the crash burst; -1 means all nodes
        int targetNodeIndex = default(0);
        // crash timing (absolute simulation time)
        double crashAt @unit(s) = default(20s);
        double resumeAfter @unit(s) = default(30s);

        // burst behavior
        int burstPackets = default(20);
        int payloadBytes = default(200);

        string packetName = default("accident_vo");

        @display("i=block/app");
}
