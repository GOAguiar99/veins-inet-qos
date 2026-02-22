package veins_qos.mac;

simple V2xEdcaFsmController
{
    parameters:
        @class(veins_qos::mac::V2xEdcaFsmController);
        double maxContinuousBlock @unit(s) = default(250ms);
        double sendingGuardTimeout @unit(s) = default(50ms);
        @display("i=block/wrxtx");
}
