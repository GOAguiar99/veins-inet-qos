package veins_qos.traffic;

import veins_qos.veins_inet.VeinsInetApplicationBase;

simple CritPacketSender extends VeinsInetApplicationBase
{
    parameters:
        @class(veins_qos::traffic::CritPacketSender);

        @signal[beTxPacketCount](type=long);
        @signal[beRxPacketCount](type=long);
        @signal[voRxPacketCount](type=long);
        @signal[beE2eDelay](type=simtime_t);
        @signal[voE2eDelay](type=simtime_t);
        @signal[beE2eDelayFromCrash](type=simtime_t);

        @statistic[beTxPackets](title="BE transmitted packets"; source=beTxPacketCount; record=count; interpolationmode=none);
        @statistic[beRxPackets](title="BE received packets"; source=beRxPacketCount; record=count; interpolationmode=none);
        @statistic[voRxPackets](title="VO received packets"; source=voRxPacketCount; record=count; interpolationmode=none);
        @statistic[beEndToEndDelay](title="BE end-to-end delay"; unit=s; source=beE2eDelay; record=vector,count,mean,min,max);
        @statistic[voEndToEndDelay](title="VO end-to-end delay"; unit=s; source=voE2eDelay; record=vector,count,mean,min,max);
        @statistic[beEndToEndDelayFromCrash](title="BE end-to-end delay from crash node"; unit=s; source=beE2eDelayFromCrash; record=vector,count,mean,min,max);

        bool enabled = default(true);

        // Redrawn on every packet. Marked volatile so ini exponential(mean)
        // is a Poisson process rather than one CBR period per vehicle.
        volatile double sendInterval @unit(s) = default(0.1s);

        int payloadBytes = default(200);

        int dscp = default(0);                 // 0=BE, 46=EF(VO)

        string packetName = default("tx");
        // Positive: keep only the first RX copy of each (src, VO sequence)
        // for the whole run (crash usefulness). The numeric value is a
        // legacy on/off threshold, not a sliding expiry window.
        double voDedupWindow @unit(s) = default(200ms);

        // Node index of the crash sender; used to split BE delay from that
        // sender. Must match CrashBurstApp.targetNodeIndex (both default 0;
        // there is no automatic coupling, change them together).
        int crashNodeIndex = default(0);

        @display("i=block/app");
    gates:
        // VeinsInetApplicationBase already defines the sockets/gates it needs;
        // no explicit gates here.
}
