package veins_qos.traffic;

import veins_qos.veins_inet.VeinsInetApplicationBase;

simple CritPacketSender extends VeinsInetApplicationBase
{
    parameters:
        @class(veins_qos::traffic::CritPacketSender);
        bool enabled = default(true);

        // periodic sending
        double sendInterval @unit(s) = default(0.1s);

        // payload sizing
        int payloadBytes = default(200);

        // QoS marking (your classifier will map DSCP->UP)
        int dscp = default(0);                 // 0=BE, 46=EF(VO)

        // packet naming (helps debugging)
        string packetName = default("tx");

        @display("i=block/app");
    gates:
        // VeinsInetApplicationBase already defines the sockets/gates it needs;
        // no explicit gates here.
}