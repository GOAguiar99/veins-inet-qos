package veins_qos.traffic;

import veins_qos.veins_inet.VeinsInetApplicationBase;

simple CrashBurstApp extends VeinsInetApplicationBase
{
    parameters:
        @class(veins_qos::traffic::CrashBurstApp);

        @signal[voTxPacketCount](type=long);
        @statistic[voTxPackets](title="VO transmitted packets"; source=voTxPacketCount; record=count; interpolationmode=none);

        // only this OMNeT++ node index triggers crash behavior; -1 means all nodes
        int targetNodeIndex = default(0);
        // crash timing (absolute simulation time)
        double crashAt @unit(s) = default(20s);
        double resumeAfter @unit(s) = default(30s);

        // VO traffic during crash window (DSCP=46)
        double sendInterval @unit(s) = default(100ms);
        int payloadBytes = default(200);
        int repeatCount = default(3);
        double repeatGap @unit(s) = default(20ms);
        double repeatJitter @unit(s) = default(5ms);

        string packetName = default("accident_vo");

        @display("i=block/app");
}
