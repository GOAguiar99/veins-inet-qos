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

        @statistic[beTxPackets](title="BE transmitted packets"; source=beTxPacketCount; record=count; interpolationmode=none);
        @statistic[beRxPackets](title="BE received packets"; source=beRxPacketCount; record=count; interpolationmode=none);
        @statistic[voRxPackets](title="VO received packets"; source=voRxPacketCount; record=count; interpolationmode=none);
        @statistic[beEndToEndDelay](title="BE end-to-end delay"; unit=s; source=beE2eDelay; record=vector,count,mean,min,max);
        @statistic[voEndToEndDelay](title="VO end-to-end delay"; unit=s; source=voE2eDelay; record=vector,count,mean,min,max);

        bool enabled = default(true);

        // periodic sending
        double sendInterval @unit(s) = default(0.1s);

        // payload sizing
        int payloadBytes = default(200);

        // QoS marking (your classifier will map DSCP->UP)
        int dscp = default(0);                 // 0=BE, 46=EF(VO)

        // packet naming (helps debugging)
        string packetName = default("tx");
        double voDedupWindow @unit(s) = default(200ms);

        @display("i=block/app");
    gates:
        // VeinsInetApplicationBase already defines the sockets/gates it needs;
        // no explicit gates here.
}
