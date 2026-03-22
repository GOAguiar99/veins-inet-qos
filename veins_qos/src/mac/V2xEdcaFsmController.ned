package veins_qos.mac;

simple V2xEdcaFsmController
{
    parameters:
        @class(veins_qos::mac::V2xEdcaFsmController);
        @signal[v2xState](type=long);
        @statistic[v2xState](title="V2X FSM state (0=LISTENING,1=BLOCKING,2=SENDING)"; source=v2xState; record=vector; interpolationmode=sample-hold);
        double maxContinuousBlock @unit(s) = default(250ms);
        double sendingGuardTimeout @unit(s) = default(50ms);
        @display("i=block/wrxtx");
}
